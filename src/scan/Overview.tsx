import { useEffect, useMemo, useState } from 'react';
import type { ReactElement, ReactNode } from 'react';
import { ArrowRight } from '@phosphor-icons/react/ArrowRight';
import { ChartPieSlice } from '@phosphor-icons/react/ChartPieSlice';
import { WarningCircle } from '@phosphor-icons/react/WarningCircle';
import './overview.css';
import { EmptyState, Notice, ResultTable, Toolbar } from '../shell/primitives';
import folderIllustration from '../../assets/illustrations/reclaim-folder.webp';
import { useScanSession } from './useScanSession';
import { categorize, categoryNames, formatBytes } from './accounting';
import { ScanSettings } from './ScanSettings';
import { VolumeUsage } from './VolumeUsage';
import type { ScanSnapshot } from './session';
import { ReviewButton } from '../cleanup/ReviewButton';

const statusLabels = {
  running: 'Scanning',
  complete: 'Scan complete',
  partial: 'Partial coverage',
  cancelled: 'Scan cancelled',
  failed: 'Scan failed',
};
function ScanResults({
  snapshot,
  retained = false,
  onInvestigate,
}: {
  snapshot: ScanSnapshot;
  retained?: boolean;
  onInvestigate: () => void;
}): ReactElement {
  const { state, session } = useScanSession();
  const [activeName, setActiveName] = useState<string | null>(null);
  const categories = useMemo(
    () => categorize(snapshot.findings),
    [snapshot.findings],
  );
  const progress = snapshot.progress;
  const excluded = snapshot.findings.filter(
    (finding) => finding.issue === 'excluded',
  ).length;
  const issues = snapshot.findings.filter(
    (finding) => finding.issue !== null && finding.issue !== 'excluded',
  );
  const readable = snapshot.findings.some(
    (finding) =>
      finding.kind === 'file' ||
      (finding.kind === 'directory' && finding.complete),
  );
  const partial =
    !snapshot.settled ||
    progress.status !== 'complete' ||
    progress.issueCount > 0;
  const attention: ReactNode[] = [];
  if (progress.status === 'cancelled')
    attention.push(
      'Cancellation preserves measured entries. Unvisited locations remain unknown. Start a fresh scan when ready.',
    );
  if (progress.status === 'failed')
    attention.push(
      'Native scan failed. Retained measurements are incomplete; review locations and start again.',
    );
  if (progress.status === 'partial')
    attention.push(
      'Partial coverage: some locations were not fully read. Unvisited descendants are unknown.',
    );
  if (snapshot.settled && !readable)
    attention.push(
      'No readable results. Review folder access or choose a different location.',
    );
  if (
    snapshot.settled &&
    readable &&
    !snapshot.findings.some((finding) => finding.kind === 'file')
  )
    attention.push(
      'No measured files in this scope. Empty directories, exclusions and unreadable entries are distinct coverage outcomes.',
    );
  if (progress.issueCount > 0)
    attention.push(
      `${progress.issueCount} coverage observations · ${excluded} excluded entries loaded · ${issues.length} access/change/limit issues loaded. Unvisited descendants are unknown.`,
    );
  return (
    <section
      className="scan-results"
      aria-label={
        retained ? 'Retained previous measurements' : 'Measured scan results'
      }
    >
      <div className="results-heading">
        <h3
          className="status-heading"
          data-status={retained ? 'stale' : progress.status}
        >
          {retained
            ? 'Previous measurements · stale during refresh'
            : statusLabels[progress.status]}
        </h3>
        <span className="secondary">Generation {progress.generation}</span>
      </div>
      <p className="secondary">
        Measured{' '}
        {new Date(
          progress.finishedAtMs ?? progress.startedAtMs,
        ).toLocaleString()}{' '}
        · {snapshot.scope.roots.length} selected locations ·{' '}
        {progress.entries.toLocaleString()} observed entries ·{' '}
        {formatBytes(progress.allocatedBytes)} allocated contribution ·{' '}
        {formatBytes(progress.logicalBytes)} logical size
        {!snapshot.settled ? ' · provisional' : ''}
      </p>
      {progress.volumes.length === 0 && (
        <Notice>
          Volume measurements unavailable so far; unknown usage is not zero.
        </Notice>
      )}
      {progress.volumes.map((accounting) => (
        <VolumeUsage
          key={accounting.volume.id}
          accounting={accounting}
          snapshot={snapshot}
          partial={partial}
          activeName={activeName}
          onActiveChange={setActiveName}
        />
      ))}
      <section className="follow-space" aria-labelledby="follow-space-heading">
        <h3 id="follow-space-heading">Follow the space</h3>
        {!snapshot.settled ? (
          <p>
            Loading measured categories… Ranking waits for final pages so rows
            stay stable.
          </p>
        ) : (
          <>
            <ResultTable caption="Measured file categories · ranked by allocated contribution">
              <thead>
                <tr>
                  <th scope="col">Category</th>
                  <th scope="col" className="numeric">
                    Allocated
                  </th>
                  <th scope="col" className="numeric">
                    Logical
                  </th>
                  <th scope="col">Investigate</th>
                </tr>
              </thead>
              <tbody>
                {categories.map((category) => (
                  <tr
                    key={category.name}
                    data-active={activeName === category.name}
                    onMouseEnter={() => setActiveName(category.name)}
                    onMouseLeave={() => setActiveName(null)}
                    onFocus={() => setActiveName(category.name)}
                    onBlur={() => setActiveName(null)}
                  >
                    <th scope="row">
                      <span
                        className={`category-swatch category-${categoryNames.indexOf(category.name)}`}
                        aria-hidden="true"
                      />
                      {category.name}
                      <span className="secondary category-count">
                        {category.findingIds.length} unique files
                      </span>
                    </th>
                    <td className="numeric">
                      {formatBytes(
                        category.allocatedBytes,
                        category.allocatedCapped,
                      )}
                    </td>
                    <td className="numeric">
                      {formatBytes(
                        category.logicalBytes,
                        category.logicalCapped,
                      )}
                    </td>
                    <td>
                      <button
                        disabled={
                          category.findingIds.length === 0 ||
                          retained ||
                          state.stale
                        }
                        onClick={() => {
                          session.investigate({
                            generation: progress.generation,
                            category: category.name,
                            findingIds: category.findingIds,
                          });
                          onInvestigate();
                        }}
                        aria-label={`Investigate ${category.name}`}
                      >
                        Explore <ArrowRight size={16} aria-hidden="true" />
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </ResultTable>
            {categories.some((category) => category.allocatedBytes > 0n) && (
              <p className="secondary">
                Start with{' '}
                {
                  categories.find((category) => category.allocatedBytes > 0n)
                    ?.name
                }
                : largest measured allocated contribution in this scope. File
                extensions describe format, not ownership or deletion safety.
              </p>
            )}
          </>
        )}
      </section>
      {attention.length > 0 && (
        <section
          className="attention-panel"
          aria-labelledby="attention-heading"
        >
          <h3 id="attention-heading">Needs attention</h3>
          <ul>
            {attention.map((message, position) => (
              <li key={position}>
                <WarningCircle size={20} weight="fill" aria-hidden="true" />
                <span>{message}</span>
              </li>
            ))}
          </ul>
        </section>
      )}
      <details className="coverage-details">
        <summary>Scope and access coverage</summary>
        <ul>
          {snapshot.scope.roots.map((root) => (
            <li className="path-text" key={root.id}>
              {root.displayPath}
            </li>
          ))}
        </ul>
        <p>
          {snapshot.scope.exclusions.length} exclusion rules frozen at start. No
          symlink descent; mount boundaries and unreadable entries reduce
          coverage.
        </p>
        {issues.slice(0, 40).map((finding) => (
          <p className="path-text" key={finding.id}>
            {finding.displayName}: {finding.issue?.replaceAll('_', ' ')}
          </p>
        ))}
        {issues.length > 40 && (
          <p>Additional issues remain available in shared findings.</p>
        )}
      </details>
      <Notice>
        Logical size differs from allocated storage. Hard links count once;
        directory aggregates are not added again. APFS clone sharing, snapshots,
        filesystem overhead and unscanned data prevent physical reconciliation
        or a reclaim estimate. Values capped at unsigned 64-bit maximum may be
        lower bounds.
      </Notice>
      {snapshot.settled && !retained && (
        <section aria-label="Review scanned locations">
          <h3>Review scanned locations</h3>
          <p>
            Check cleanup limitations for an exact scanned location. Recognition
            and size do not make its contents removable.
          </p>
          {snapshot.findings
            .filter((finding) => finding.parentId === null)
            .map((finding) => (
              <div key={finding.id} className="review-target-heading">
                <span className="path-text">{finding.displayName}</span>
                <ReviewButton
                  disabled={state.stale}
                  selection={{
                    source: 'filesystem',
                    generation: progress.generation,
                    findingId: finding.id,
                  }}
                />
              </div>
            ))}
        </section>
      )}
    </section>
  );
}

export function Overview({
  onInvestigate,
}: {
  onInvestigate: () => void;
}): ReactElement {
  const { state, session } = useScanSession();
  const [editing, setEditing] = useState(false);
  const [clock, setClock] = useState(() => Date.now());
  const snapshot = state.snapshot;
  const running = snapshot?.progress.status === 'running';
  useEffect(() => {
    if (!running) return;
    const timer = setInterval(() => setClock(Date.now()), 1000);
    return () => clearInterval(timer);
  }, [running]);
  const elapsed = snapshot
    ? Math.max(
        0,
        Math.floor(
          ((snapshot.progress.finishedAtMs ?? clock) -
            snapshot.progress.startedAtMs) /
            1000,
        ),
      )
    : 0;
  const phase = snapshot
    ? snapshot.settled
      ? statusLabels[snapshot.progress.status]
      : running
        ? 'Measuring metadata'
        : 'Loading final measurements'
    : 'Scan not started';
  return (
    <div className="overview-content">
      {!snapshot && (
        <div className="section-introduction introduction-overview">
          <EmptyState
            icon={<ChartPieSlice size={28} />}
            title="Understand your space"
          >
            <p>
              Choose where to look. Follow measured storage to the files behind
              it, with unknowns kept in view.
            </p>
            <p>No scan starts until you choose Start scan.</p>
          </EmptyState>
          <img
            className="folder-illustration"
            src={folderIllustration}
            width={1200}
            height={1200}
            alt=""
            aria-hidden="true"
          />
        </div>
      )}
      {state.error && (
        <div role="alert">
          <Notice>{state.error}</Notice>
          {snapshot && !snapshot.settled && (
            <button onClick={session.resume}>Retry connection</button>
          )}
        </div>
      )}
      {state.stale && (
        <Notice>
          Stale measurements: settings changed or connection lost. Retained
          results describe their original scope and time. Refresh deliberately.
        </Notice>
      )}
      {snapshot && (
        <header className="overview-heading">
          <h2>Where did the space go?</h2>
          <p
            className={snapshot.settled ? 'visually-hidden' : 'scan-phase'}
            role="status"
          >
            {phase}
          </p>
          <p className="secondary">{elapsed}s elapsed</p>
        </header>
      )}
      <Toolbar label="Scan controls">
        <button
          className="primary-action"
          disabled={
            state.loading ||
            state.busy ||
            running ||
            (!!snapshot && !snapshot.settled) ||
            state.preferences.roots.length === 0
          }
          onClick={() => void session.start()}
        >
          {state.busy
            ? 'Preparing scan…'
            : snapshot
              ? 'Refresh scan'
              : 'Start scan'}
        </button>
        {running && (
          <button
            onClick={() => void session.cancel()}
            disabled={state.cancelling}
          >
            {state.cancelling ? 'Cancelling…' : 'Cancel scan'}
          </button>
        )}
        {snapshot && (
          <button onClick={() => setEditing(!editing)} aria-expanded={editing}>
            Edit next scan
          </button>
        )}
      </Toolbar>
      {!snapshot && (
        <p className="scan-phase" role="status">
          {phase}
        </p>
      )}
      {running && snapshot && (
        <section className="progress-card" aria-label="Scan progress">
          <div className="progress-track" aria-hidden="true">
            <span />
          </div>
          <p>
            {snapshot.progress.currentLocation
              ? `Reading ${snapshot.progress.currentLocation}`
              : 'Preparing locations'}
          </p>
          <p className="secondary">
            {snapshot.progress.entries.toLocaleString()} entries observed so
            far. Total work unknown; no percent estimate.
          </p>
        </section>
      )}
      {(!snapshot || editing) && <ScanSettings />}
      {snapshot && (
        <ScanResults snapshot={snapshot} onInvestigate={onInvestigate} />
      )}
      {state.previous && snapshot && !snapshot.settled && (
        <ScanResults
          snapshot={state.previous}
          retained
          onInvestigate={onInvestigate}
        />
      )}
    </div>
  );
}
