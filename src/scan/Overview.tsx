import { useEffect, useMemo, useState } from 'react';
import type { ReactElement, ReactNode } from 'react';
import { ArrowRight } from '@phosphor-icons/react/ArrowRight';
import { ChartPieSlice } from '@phosphor-icons/react/ChartPieSlice';
import { CaretRight } from '@phosphor-icons/react/CaretRight';
import { WarningCircle } from '@phosphor-icons/react/WarningCircle';
import './overview.css';
import { EmptyState, Notice, ResultTable } from '../shell/primitives';
import folderIllustration from '../../assets/illustrations/reclaim-folder.webp';
import { useScanSession } from './useScanSession';
import { categorize, categoryNames, formatBytes } from './accounting';
import { ScanSettings } from './ScanSettings';
import { VolumeUsage } from './VolumeUsage';
import type { ScanSnapshot } from './session';
import { ReviewButton } from '../cleanup/ReviewButton';
import { AccessGuidance, OpenSettingsButton } from '../access/AccessGuidance';
import { useAccess } from '../access/useAccess';

const statusLabels = {
  running: 'Scanning',
  complete: 'Scan complete',
  partial: 'Finished with gaps',
  cancelled: 'Scan cancelled',
  failed: 'Scan failed',
};
function pluralize(count: number, noun: string): string {
  return `${count} ${noun}${count === 1 ? '' : 's'}`;
}

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
  const progress = snapshot.progress;
  const summary = snapshot.summary;
  const topFiles = useMemo(() => summary?.topFiles ?? [], [summary]);
  const categories = useMemo(
    () => categorize(progress.volumes, topFiles),
    [progress.volumes, topFiles],
  );
  const issueGroups = summary?.issues ?? [];
  const groupCount = (issue: string): number =>
    issueGroups.find((group) => group.issue === issue)?.count ?? 0;
  const excluded = groupCount('excluded');
  const denied =
    issueGroups.find((group) => group.issue === 'permission_denied')
      ?.examples ?? [];
  const issues = issueGroups
    .filter((group) => group.issue !== 'excluded')
    .flatMap((group) => group.examples);
  const issueTotal = issueGroups
    .filter((group) => group.issue !== 'excluded')
    .reduce((total, group) => total + group.count, 0);
  const measuredFiles = categories.reduce(
    (total, category) => total + category.count,
    0,
  );
  const access = useAccess();
  const readable =
    measuredFiles > 0 ||
    (summary?.roots ?? []).some(
      (root) => root.kind === 'directory' && root.complete,
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
      'Some locations were not fully read. Unvisited descendants are unknown.',
    );
  if (progress.sharingUntracked)
    attention.push(
      'Sharing between hard-linked files could not be fully tracked in this scan. Allocated totals may count some shared data more than once; the true figure is unknown.',
    );
  if (snapshot.settled && !readable)
    attention.push(
      'No readable results. Review folder access or choose a different location.',
    );
  if (snapshot.settled && readable && measuredFiles === 0)
    attention.push(
      'No measured files in this scope. Empty directories, exclusions and unreadable entries are distinct coverage outcomes.',
    );
  if (progress.issueCount > 0)
    attention.push(
      <>
        <dl className="attention-stats">
          <div>
            <dt>Coverage observations</dt>
            <dd>{progress.issueCount}</dd>
          </div>
          <div>
            <dt>Excluded entries</dt>
            <dd>{excluded}</dd>
          </div>
          <div>
            <dt>Access or change issues</dt>
            <dd>{issueTotal}</dd>
          </div>
        </dl>
        {progress.status === 'complete' && (
          <span>Unvisited descendants are unknown.</span>
        )}
      </>,
    );
  return (
    <section
      className="scan-results"
      data-volumes={progress.volumes.length}
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
      <p className="secondary results-meta">
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
          topFiles={topFiles}
          settled={snapshot.settled}
          partial={partial}
          activeName={activeName}
          onActiveChange={setActiveName}
        />
      ))}
      <section className="follow-space" aria-labelledby="follow-space-heading">
        <h3 id="follow-space-heading">Follow the space</h3>
        <>
          <ResultTable
            caption={
              snapshot.settled
                ? 'Measured file categories · ranked by allocated contribution'
                : 'Live category totals · provisional until the scan finishes'
            }
          >
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
                      {category.count.toLocaleString()} unique files
                    </span>
                  </th>
                  <td className="numeric">
                    {formatBytes(
                      category.allocatedBytes,
                      category.allocatedCapped,
                    )}
                  </td>
                  <td className="numeric">
                    {formatBytes(category.logicalBytes, category.logicalCapped)}
                  </td>
                  <td>
                    <button
                      disabled={
                        !snapshot.settled ||
                        category.files.length === 0 ||
                        retained ||
                        state.stale
                      }
                      onClick={() => {
                        session.investigate({
                          generation: progress.generation,
                          category: category.name,
                          findingIds: category.files.flatMap((file) =>
                            file.findingId === null ? [] : [file.findingId],
                          ),
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
          {snapshot.settled && (
            <p className="secondary">
              Category totals cover every measured file. Browsing lists items of{' '}
              {formatBytes(progress.retention.thresholdBytes)} or more; smaller
              items are summarized in their folders.
            </p>
          )}
          {snapshot.settled &&
            categories.some((category) => category.allocatedBytes > 0n) && (
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
      </section>
      {denied.length > 0 && <AccessGuidance context="denied" />}
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
                <div>{message}</div>
              </li>
            ))}
          </ul>
        </section>
      )}
      <details className="coverage-details">
        <summary>
          <CaretRight size={16} aria-hidden="true" />
          Scope and access coverage
        </summary>
        <div className="coverage-details-body">
          <ul>
            {snapshot.scope.roots.map((root) => (
              <li className="path-text" key={root.id}>
                {root.displayPath}
              </li>
            ))}
          </ul>
          <p>
            {snapshot.scope.exclusions.length} exclusion rules frozen at start.
            No symlink descent; mount boundaries and unreadable entries reduce
            coverage.
          </p>
          {denied.length > 0 && (
            <div className="access-denied-list">
              <h4>Denied by macOS</h4>
              <p>
                macOS refused read access to these locations, so they are
                unknown, not empty.
                {access?.status !== 'granted' &&
                  ' Full Disk Access may let a new scan read them.'}
              </p>
              <ul aria-label="Locations macOS denied">
                {denied.slice(0, 40).map((finding) => (
                  <li className="path-text" key={finding.id}>
                    {finding.displayName}
                  </li>
                ))}
              </ul>
              {access && access.status !== 'granted' && (
                <div className="access-actions">
                  <OpenSettingsButton />
                </div>
              )}
            </div>
          )}
          {issues
            .filter((finding) => finding.issue !== 'permission_denied')
            .slice(0, 40)
            .map((finding) => (
              <p className="path-text" key={finding.id}>
                {finding.displayName}: {finding.issue?.replaceAll('_', ' ')}
              </p>
            ))}
          {issueTotal > issues.length && (
            <p>
              Showing {issues.length.toLocaleString()} of{' '}
              {issueTotal.toLocaleString()} observations.
            </p>
          )}
        </div>
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
          {(summary?.roots ?? []).map((finding) => (
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
  const refreshBlocked = running || (!!snapshot && !snapshot.settled);
  const phase = snapshot
    ? snapshot.settled
      ? statusLabels[snapshot.progress.status]
      : running
        ? 'Measuring metadata'
        : 'Loading final measurements'
    : 'Scan not started';
  const scanHero = (
    <section className="scan-hero" aria-label="Scan controls">
      {running ? (
        <button
          className="scan-hero-button"
          onClick={() => void session.cancel()}
          disabled={state.cancelling}
        >
          {state.cancelling ? 'Cancelling…' : 'Cancel scan'}
        </button>
      ) : (
        <button
          className="primary-action scan-hero-button"
          disabled={
            state.loading ||
            state.busy ||
            refreshBlocked ||
            state.preferences.roots.length === 0
          }
          aria-describedby={refreshBlocked ? 'refresh-hint' : undefined}
          onClick={() => void session.start()}
        >
          {state.busy
            ? 'Preparing scan…'
            : snapshot
              ? 'Scan again'
              : 'Start scan'}
        </button>
      )}
      <p className="scan-scope-summary secondary">
        {state.loading
          ? 'Loading saved settings…'
          : `${pluralize(state.preferences.roots.length, 'location')} · ${pluralize(state.preferences.exclusions.length, 'exclusion')}`}
        <button
          className="scan-scope-edit"
          onClick={() => setEditing(!editing)}
          aria-expanded={editing}
          aria-controls="scan-settings-panel"
        >
          Edit
        </button>
      </p>
      {refreshBlocked && !running && (
        <span id="refresh-hint" className="toolbar-hint secondary">
          Available when the current scan finishes.
        </span>
      )}
    </section>
  );
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
            {scanHero}
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
      {snapshot && scanHero}
      {!snapshot && <AccessGuidance context="first-run" />}
      {!snapshot && (
        <p className="visually-hidden" role="status">
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
            {snapshot.progress.entries.toLocaleString()} entries observed so far
            · {formatBytes(snapshot.progress.allocatedBytes)} allocated. Total
            work unknown; no percent estimate.
          </p>
        </section>
      )}
      {editing && (
        <div id="scan-settings-panel">
          <ScanSettings />
        </div>
      )}
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
