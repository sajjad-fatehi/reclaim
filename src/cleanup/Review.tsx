import { ClipboardText } from '@phosphor-icons/react/ClipboardText';
import { useEffect, useRef, useState } from 'react';
import type { ReactElement } from 'react';
import { formatBytes } from '../scan/accounting';
import { useScanSession } from '../scan/useScanSession';
import { Dialog, EmptyState, Notice, Toolbar } from '../shell/primitives';
import { nativeCleanupApi, selectionKey } from './contract';
import type {
  CleanupApi,
  Execution,
  HistoryResponse,
  Review as ReviewReport,
} from './contract';
import { useReview } from './useReview';

export function Review({
  active,
  api = nativeCleanupApi,
}: {
  active: boolean;
  api?: CleanupApi | undefined;
}): ReactElement {
  const selection = useReview();
  const { state } = useScanSession();
  const [checked, setChecked] = useState<{
    key: string;
    report: ReviewReport;
  } | null>(null);
  const [execution, setExecution] = useState<Execution | null>(null);
  const [executing, setExecuting] = useState(false);
  const [confirmation, setConfirmation] = useState(false);
  const [acknowledged, setAcknowledged] = useState(false);
  const cancelReference = useRef<HTMLButtonElement>(null);
  const executingReference = useRef(false);
  const [history, setHistory] = useState<HistoryResponse | null>(null);
  const [error, setError] = useState('');
  const [historyError, setHistoryError] = useState('');
  const [busy, setBusy] = useState(false);
  const [historyBusy, setHistoryBusy] = useState(false);
  const [revision, setRevision] = useState(0);
  const pending = useRef(false);
  const clearing = useRef(false);
  const generation = state.snapshot?.progress.generation;
  const selections = selection?.selections ?? [];
  const key = JSON.stringify(selections.map(selectionKey));
  const report = checked?.key === key ? checked.report : null;
  const stale =
    state.stale ||
    selections.some(
      (entry) =>
        entry.source === 'filesystem' && entry.generation !== generation,
    );
  useEffect(() => {
    if (!active) return;
    let current = true;
    void api
      .status()
      .then((next) => {
        if (current && next) {
          setExecution(next);
          setExecuting(next.running);
        }
      })
      .catch(() => undefined);
    void api
      .history()
      .then((response) => {
        if (current) {
          setHistory(response);
          setHistoryError('');
        }
      })
      .catch(() => {
        if (current)
          setHistoryError(
            'History unavailable. Retry history; no removal was replayed.',
          );
      });
    return () => {
      current = false;
    };
  }, [active, api, revision]);
  useEffect(() => {
    if (!executing) return;
    let current = true;
    const timer = setInterval(() => {
      void api
        .status()
        .then((next) => {
          if (current && next?.running) setExecution(next);
        })
        .catch(() => undefined);
    }, 200);
    return () => {
      current = false;
      clearInterval(timer);
    };
  }, [executing, api]);
  async function executePlan(): Promise<void> {
    if (!report?.planId || executingReference.current || stale) return;
    if (
      report.items.some(
        (item) => item.method !== null && item.method !== 'trash',
      ) &&
      !acknowledged
    )
      return;
    executingReference.current = true;
    setExecuting(true);
    setConfirmation(false);
    setError('');
    try {
      const result = await api.execute(report.planId);
      if (result.planId !== report.planId)
        throw new Error('Wrong execution response');
      setExecution(result);
      setChecked(null);
      setRevision((current) => current + 1);
    } catch {
      setChecked(null);
      setError(
        'Execution response unavailable. Some operations may have completed. Inspect native history and targets before a fresh review; no automatic retry.',
      );
      setRevision((current) => current + 1);
    } finally {
      executingReference.current = false;
      setExecuting(false);
    }
  }
  async function checkSelection(): Promise<void> {
    if (
      pending.current ||
      executingReference.current ||
      !selections.length ||
      stale
    )
      return;
    pending.current = true;
    setBusy(true);
    setChecked(null);
    setConfirmation(false);
    setError('');
    try {
      const next = await api.review(selections);
      const expected = new Set(selections.map(selectionKey));
      if (
        next.items.length !== expected.size ||
        next.items.some(
          (item) => !expected.delete(selectionKey(item.selection)),
        ) ||
        expected.size
      )
        throw new Error('Mismatched review response');
      setChecked({ key, report: next });
    } catch {
      setChecked(null);
      setError(
        'Eligibility unavailable for this selection. Remove stale items or rescan, then retry. No removal dispatched by this review.',
      );
    } finally {
      pending.current = false;
      setBusy(false);
    }
  }
  async function clearHistory(): Promise<void> {
    if (clearing.current || executingReference.current) return;
    clearing.current = true;
    setHistoryBusy(true);
    try {
      await api.clearHistory();
      setHistory({ history: { operations: [] }, unavailable: false });
      setHistoryError('');
    } catch {
      setHistoryError(
        'History could not be cleared. Retained records may remain; retry. No removal dispatched by this review.',
      );
    } finally {
      clearing.current = false;
      setHistoryBusy(false);
    }
  }
  return (
    <div className="cleanup-review">
      <Notice>
        Only explicitly allowlisted targets can be removed. Review exact targets
        and consequences, then confirm. Native checks run immediately before
        removal; concurrent changes between checks and the platform/tool call
        remain possible. Trash is never replaced with permanent deletion.
      </Notice>
      <section aria-label="Review selection">
        <h2>
          Review selection{' '}
          <span className="secondary">· {selections.length} / 100</span>
        </h2>
        {selections.length === 0 ? (
          <EmptyState
            icon={<ClipboardText size={28} />}
            title="Choose what to understand"
          >
            <p>
              Open item details in Explore or Development, then choose Review
              cleanup options. Overview links to those same findings.
            </p>
          </EmptyState>
        ) : (
          <>
            <Toolbar label="Review actions">
              <button
                className="primary"
                disabled={
                  busy || executing || stale || !state.snapshot?.settled
                }
                onClick={() => void checkSelection()}
              >
                {busy ? 'Checking eligibility…' : 'Check eligibility'}
              </button>
              <button
                disabled={busy || executing}
                onClick={() => selection?.clear()}
              >
                Clear selection
              </button>
              {report?.planId && (
                <button
                  className="primary"
                  disabled={executing || stale}
                  onClick={() => {
                    setAcknowledged(false);
                    setConfirmation(true);
                  }}
                >
                  Review confirmation
                </button>
              )}
            </Toolbar>
            {stale && (
              <p role="alert">
                Selection contains stale measurements. Clear selection and
                choose findings from a fresh scan.
              </p>
            )}
            <p>
              Selection stays explicit across navigation and filters. Parent and
              child measurements may overlap; no total is presented as
              recoverable space.
            </p>
            <ul className="review-selection">
              {selections.map((entry) => {
                const item = report?.items.find(
                  (item) =>
                    selectionKey(item.selection) === selectionKey(entry),
                );
                return (
                  <li key={selectionKey(entry)}>
                    <div className="review-target-heading">
                      <h3 className="path-text">
                        {item?.target ??
                          `${entry.source} finding ${entry.findingId}`}
                      </h3>
                      <button
                        disabled={busy || executing}
                        onClick={() => selection?.remove(entry)}
                      >
                        Remove from review
                      </button>
                    </div>
                    {!item ? (
                      <p>
                        Eligibility not checked. Selection grants no cleanup
                        authority.
                      </p>
                    ) : (
                      <>
                        <p className="review-availability">
                          {item.changed
                            ? 'Changed · removal blocked'
                            : item.method
                              ? 'Eligible after confirmation'
                              : 'Removal unavailable'}
                        </p>
                        <p>{item.unavailableReason}</p>
                        <dl className="inspection-facts">
                          <dt>Logical measurement</dt>
                          <dd>
                            {formatBytes(
                              item.logicalBytes === 'unknown'
                                ? null
                                : item.logicalBytes,
                            )}
                          </dd>
                          <dt>Allocated contribution</dt>
                          <dd>{formatBytes(item.allocatedBytes)}</dd>
                          <dt>Measured</dt>
                          <dd>
                            {item.measuredAtMs === null
                              ? 'Unavailable'
                              : new Date(item.measuredAtMs).toLocaleString()}
                          </dd>
                          <dt>Owner / confidence</dt>
                          <dd>
                            {item.artifact
                              ? `${item.artifact.family.replaceAll('_', ' ')} · ${item.artifact.confidence}`
                              : 'Unknown'}
                          </dd>
                          <dt>Method / recovery</dt>
                          <dd>
                            {item.method === 'trash'
                              ? 'macOS Trash · recovery through macOS Trash / Put Back where available'
                              : item.method
                                ? `${item.method.replaceAll('_', ' ')} · irreversible, no Trash recovery`
                                : 'Unavailable. No operation authorized.'}
                          </dd>
                        </dl>
                        <p>
                          {item.artifact?.consequence ??
                            'Removal consequences unknown. Preserve source, personal and persistent data.'}
                        </p>
                        {item.artifact && (
                          <ul>
                            {item.artifact.evidence.map((evidence) => (
                              <li
                                key={`${evidence.findingId}:${evidence.method}`}
                              >
                                {evidence.method.replaceAll('_', ' ')} · finding{' '}
                                {evidence.findingId}
                              </li>
                            ))}
                          </ul>
                        )}
                      </>
                    )}
                  </li>
                );
              })}
            </ul>
            {report && (
              <p className="secondary">
                Checked {new Date(report.checkedAtMs).toLocaleString()}.
                Eligible targets require separate confirmation of a single-use
                plan. APFS sharing, snapshots and physically recoverable space
                remain unknown.
              </p>
            )}
          </>
        )}
        <p role="status">{selection?.message}</p>
        {error && <p role="alert">{error}</p>}
      </section>
      <section aria-label="Execution outcomes" aria-live="polite">
        {executing && (
          <>
            <h2>Cleanup in progress</h2>
            <p>
              Cancellation stops pending items; completed work is not undone.
            </p>
            <button
              onClick={() => {
                void api
                  .cancel()
                  .catch(() =>
                    setError(
                      'Cancellation request failed. Inspect progress before retrying cancellation.',
                    ),
                  );
              }}
            >
              Cancel pending items
            </button>
          </>
        )}
        {execution && (
          <>
            <h2>
              {execution.running ? 'Current outcomes' : 'Cleanup outcomes'}
            </h2>
            <ul>
              {execution.operation.items.map((item, index) => (
                <li key={`${execution.operation.id}:${index}`}>
                  <strong className="path-text">{item.label}</strong> ·{' '}
                  {item.outcome}
                  <p>{item.explanation}</p>
                </li>
              ))}
            </ul>
            {execution.persistenceError && (
              <p role="alert">
                History persistence failed. Pending work stopped; inspect these
                outcomes and targets before another review. Restart may retain
                unknown outcomes.
              </p>
            )}
            <p>
              Retry only by checking a fresh plan. Removed targets need a new
              scan; Unknown outcomes require inspection first.
            </p>
          </>
        )}
      </section>
      <Dialog
        title="Confirm exact cleanup targets"
        open={confirmation}
        onClose={() => setConfirmation(false)}
        initialFocus={cancelReference}
      >
        <p>
          {report?.items.some(
            (item) => item.method !== null && item.method !== 'trash',
          )
            ? 'Irreversible tool removal. No macOS Trash recovery.'
            : 'Move these exact directories, including local edits, to macOS Trash. Recover using macOS Trash / Put Back where available.'}
        </p>
        <ul>
          {report?.items
            .filter((item) => item.method !== null)
            .map((item) => (
              <li className="path-text" key={selectionKey(item.selection)}>
                {item.target}
                <p>{item.unavailableReason}</p>
              </li>
            ))}
        </ul>
        <p>
          Measurements are not guaranteed reclaimed physical space. Immediate
          checks cannot eliminate changes between check and operation.
        </p>
        {report?.items.some(
          (item) => item.method !== null && item.method !== 'trash',
        ) && (
          <label>
            <input
              type="checkbox"
              checked={acknowledged}
              onChange={(event) => setAcknowledged(event.target.checked)}
            />
            I understand this tool operation is irreversible and may have
            partial effects on error.
          </label>
        )}
        <Toolbar label="Cleanup confirmation">
          <button ref={cancelReference} onClick={() => setConfirmation(false)}>
            Cancel
          </button>
          <button
            className="primary"
            disabled={
              executing ||
              stale ||
              !report?.planId ||
              (report.items.some(
                (item) => item.method !== null && item.method !== 'trash',
              ) &&
                !acknowledged)
            }
            onKeyDown={(event) => {
              if (
                event.key === 'Enter' &&
                report?.items.some(
                  (item) => item.method !== null && item.method !== 'trash',
                )
              )
                event.preventDefault();
            }}
            onClick={() => void executePlan()}
          >
            Confirm selected removal
          </button>
        </Toolbar>
      </Dialog>
      <section aria-label="Operation history">
        <h2>Operation history</h2>
        <p>
          Private local records, limited to 50 operations. Interrupted outcomes
          remain unknown; restarting never replays removals. Eligibility reviews
          are not removal operations.
        </p>
        <Toolbar label="History actions">
          <button
            disabled={historyBusy || executing}
            onClick={() => setRevision((current) => current + 1)}
          >
            Retry history
          </button>
          <button
            disabled={
              historyBusy ||
              executing ||
              (!history?.unavailable && !history?.history?.operations.length)
            }
            onClick={() => void clearHistory()}
          >
            {historyBusy ? 'Clearing history…' : 'Clear history'}
          </button>
        </Toolbar>
        <p className="secondary">
          Clearing history removes local records only. It cannot recover or
          remove targets.
        </p>
        {historyError && <p role="alert">{historyError}</p>}
        {history?.unavailable && (
          <Notice>
            History corrupt, inaccessible or invalid. Existing records were not
            replaced. Retry or explicitly clear history.
          </Notice>
        )}
        {!history && !historyError && <p>Loading history…</p>}
        {history?.history?.operations.length === 0 && (
          <p>No recorded operations.</p>
        )}
        {history?.history?.operations.map((operation) => (
          <article key={operation.id} className="history-operation">
            <h3>{new Date(operation.startedAtMs).toLocaleString()}</h3>
            <p>
              {operation.finishedAtMs === null
                ? 'Interrupted or unfinished · inspect targets before a new review'
                : `Finished ${new Date(operation.finishedAtMs).toLocaleString()}`}
            </p>
            <ul>
              {operation.items.map((item, index) => (
                <li key={`${operation.id}:${index}`}>
                  <strong className="path-text">{item.label}</strong> ·{' '}
                  {item.method.replaceAll('_', ' ')} ·{' '}
                  {item.outcome === 'pending' ? 'unknown' : item.outcome}
                  <p>
                    {item.outcome === 'pending'
                      ? 'Interrupted. Status unknown; no automatic retry.'
                      : item.explanation}
                  </p>
                </li>
              ))}
            </ul>
            <p>
              Volume free-space observations (not attributable reclaimed space):{' '}
              {operation.volumeObservations?.length
                ? operation.volumeObservations
                    .map(
                      (volume) =>
                        `${volume.volumeId}: ${formatBytes(volume.beforeFreeBytes)} → ${formatBytes(volume.afterFreeBytes)}`,
                    )
                    .join('; ')
                : 'Unavailable'}
              . APFS sharing/snapshots and concurrent activity limit
              attribution. Apparent sizes are not reclaimed space.
            </p>
          </article>
        ))}
      </section>
    </div>
  );
}
