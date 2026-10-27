import { ClipboardText } from '@phosphor-icons/react/ClipboardText';
import { useEffect, useRef, useState } from 'react';
import type { ReactElement } from 'react';
import { formatBytes } from '../scan/accounting';
import { useScanSession } from '../scan/useScanSession';
import { EmptyState, Notice, Toolbar } from '../shell/primitives';
import { nativeCleanupApi, selectionKey } from './contract';
import type {
  CleanupApi,
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
  async function checkSelection(): Promise<void> {
    if (pending.current || !selections.length || stale) return;
    pending.current = true;
    setBusy(true);
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
        'Eligibility unavailable for this selection. Remove stale items or rescan, then retry. Files unchanged.',
      );
    } finally {
      pending.current = false;
      setBusy(false);
    }
  }
  async function clearHistory(): Promise<void> {
    if (clearing.current) return;
    clearing.current = true;
    setHistoryBusy(true);
    try {
      await api.clearHistory();
      setHistory({ history: { operations: [] }, unavailable: false });
      setHistoryError('');
    } catch {
      setHistoryError(
        'History could not be cleared. Retained records may remain; retry. Files unchanged.',
      );
    } finally {
      clearing.current = false;
      setHistoryBusy(false);
    }
  }
  return (
    <div className="cleanup-review">
      <Notice>
        Cleanup unavailable in this build. Filesystem Trash, Git worktree
        removal and Docker image removal lack proven concurrent safety. Review
        explains each target; no removal, recovery or freed-space claim is
        offered.
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
                disabled={busy || stale || !state.snapshot?.settled}
                onClick={() => void checkSelection()}
              >
                {busy ? 'Checking eligibility…' : 'Check eligibility'}
              </button>
              <button disabled={busy} onClick={() => selection?.clear()}>
                Clear selection
              </button>
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
                        disabled={busy}
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
                          <dd>Unavailable. No operation authorized.</dd>
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
                Checked {new Date(report.checkedAtMs).toLocaleString()}. This
                report is not an executable plan. APFS sharing, snapshots and
                physically recoverable space remain unknown.
              </p>
            )}
          </>
        )}
        <p role="status">{selection?.message}</p>
        {error && <p role="alert">{error}</p>}
      </section>
      <section aria-label="Operation history">
        <h2>Operation history</h2>
        <p>
          Private local records, limited to 50 operations. Interrupted outcomes
          remain unknown; restarting never replays removals. Eligibility reviews
          are not removal operations.
        </p>
        <Toolbar label="History actions">
          <button
            disabled={historyBusy}
            onClick={() => setRevision((current) => current + 1)}
          >
            Retry history
          </button>
          <button
            disabled={
              historyBusy ||
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
          <p>No recorded operations. No space reclaimed by this build.</p>
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
              Measured volume free-space change unavailable. Apparent sizes are
              not reclaimed space.
            </p>
          </article>
        ))}
      </section>
    </div>
  );
}
