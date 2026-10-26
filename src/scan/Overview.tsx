import { useEffect, useMemo, useState } from 'react';
import type { ReactElement } from 'react';
import { ArrowRight } from '@phosphor-icons/react/ArrowRight';
import { ChartPieSlice } from '@phosphor-icons/react/ChartPieSlice';
import { EmptyState, Notice, ResultTable, Toolbar } from '../shell/primitives';
import folderIllustration from '../../assets/illustrations/reclaim-folder.webp';
import { useScanSession } from './useScanSession';
import {
  categorize,
  categoryNames,
  formatBytes,
  segmentWidth,
} from './accounting';
import { ScanSettings } from './ScanSettings';
import type { ScanSnapshot } from './session';

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
  return (
    <section
      className="scan-results"
      aria-label={
        retained ? 'Retained previous measurements' : 'Measured scan results'
      }
    >
      <div className="results-heading">
        <h2>
          {retained
            ? 'Previous measurements · stale during refresh'
            : statusLabels[progress.status]}
        </h2>
        <span>Generation {progress.generation}</span>
      </div>
      <p className="secondary">
        Measured{' '}
        {new Date(
          progress.finishedAtMs ?? progress.startedAtMs,
        ).toLocaleString()}{' '}
        · {snapshot.scope.roots.length} selected locations ·{' '}
        {progress.entries.toLocaleString()} observed entries
      </p>
      <p>
        {formatBytes(progress.allocatedBytes)} allocated contribution ·{' '}
        {formatBytes(progress.logicalBytes)} logical size
        {!snapshot.settled ? ' · provisional' : ''}
      </p>
      {progress.status === 'cancelled' && (
        <Notice>
          Cancellation preserves measured entries. Unvisited locations remain
          unknown. Start a fresh scan when ready.
        </Notice>
      )}
      {progress.status === 'failed' && (
        <Notice>
          Native scan failed. Retained measurements are incomplete; review
          locations and start again.
        </Notice>
      )}
      {snapshot.settled && !readable && (
        <Notice>
          No readable results. Review folder access or choose a different
          location.
        </Notice>
      )}
      {snapshot.settled &&
        readable &&
        !snapshot.findings.some((finding) => finding.kind === 'file') && (
          <Notice>
            No measured files in this scope. Empty directories, exclusions and
            unreadable entries are distinct coverage outcomes.
          </Notice>
        )}
      {progress.issueCount > 0 && (
        <Notice>
          {progress.issueCount} coverage observations · {excluded} excluded
          entries loaded · {issues.length} access/change/limit issues loaded.
          Unvisited descendants are unknown.
        </Notice>
      )}
      <details>
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
      <h2>Volume usage</h2>
      {progress.volumes.length === 0 && (
        <Notice>
          Volume measurements unavailable so far; unknown usage is not zero.
        </Notice>
      )}
      {progress.volumes.map((accounting) => {
        const { volume } = accounting;
        const volumeCategories = snapshot.settled
          ? categorize(
              snapshot.findings.filter((finding) =>
                finding.identity?.startsWith(`${volume.id}:`),
              ),
            )
          : [];
        const categorizedBytes = volumeCategories.reduce(
          (total, category) => total + category.allocatedBytes,
          0n,
        );
        const matchingCategories =
          snapshot.settled &&
          categorizedBytes === BigInt(accounting.scopeAllocatedBytes);
        const total = volume.totalBytes;
        const remainder = accounting.unscannedOrUnexplainedBytes;
        const consistent =
          total !== null &&
          BigInt(total) > 0n &&
          volume.freeBytes !== null &&
          remainder !== null &&
          BigInt(accounting.scopeAllocatedBytes) +
            BigInt(remainder) +
            BigInt(volume.freeBytes) ===
            BigInt(total);
        return (
          <section
            className="volume-usage"
            key={volume.id}
            aria-label={`Volume ${volume.id} usage`}
          >
            <div className="results-heading">
              <h3>Volume {volume.id}</h3>
              <span>{formatBytes(total)} total</span>
            </div>
            {consistent &&
            total !== null &&
            remainder !== null &&
            volume.freeBytes !== null ? (
              <div className="usage-strip" aria-hidden="true">
                {matchingCategories ? (
                  volumeCategories.map((category) => (
                    <span
                      key={category.name}
                      className={`segment-category category-${categoryNames.indexOf(category.name)}`}
                      style={{
                        width: segmentWidth(
                          category.allocatedBytes.toString(),
                          total,
                        ),
                      }}
                    />
                  ))
                ) : (
                  <span
                    className="segment-measured"
                    style={{
                      width: segmentWidth(
                        accounting.scopeAllocatedBytes,
                        total,
                      ),
                    }}
                  />
                )}
                <span
                  className="segment-unknown"
                  style={{ width: segmentWidth(remainder, total) }}
                />
                <span
                  className="segment-free"
                  style={{ width: segmentWidth(volume.freeBytes, total) }}
                />
              </div>
            ) : (
              <p>
                Usage segments unavailable: missing or inconsistent physical
                accounting.
              </p>
            )}
            {matchingCategories && (
              <p className="secondary">
                Measured categories:{' '}
                {volumeCategories
                  .filter((category) => category.allocatedBytes > 0n)
                  .map(
                    (category) =>
                      `${category.name} ${formatBytes(category.allocatedBytes)}`,
                  )
                  .join(' · ') || 'No allocated file contributions'}
                .
              </p>
            )}
            <dl className="volume-legend">
              <div>
                <dt>Measured allocated</dt>
                <dd>{formatBytes(accounting.scopeAllocatedBytes)}</dd>
              </div>
              <div>
                <dt>Unscanned / unexplained</dt>
                <dd>{formatBytes(remainder)}</dd>
              </div>
              <div>
                <dt>Free blocks</dt>
                <dd>{formatBytes(volume.freeBytes)}</dd>
              </div>
              <div>
                <dt>Available to you</dt>
                <dd>{formatBytes(volume.availableBytes)}</dd>
              </div>
            </dl>
            <p className="secondary">
              Volume sampled{' '}
              {new Date(volume.measuredAtMs).toLocaleTimeString()}.{' '}
              {volume.metricIssue &&
                `Metric issue: ${volume.metricIssue.replaceAll('_', ' ')}.`}
            </p>
          </section>
        );
      })}
      <h2>Follow the space</h2>
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
                <tr key={category.name}>
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
                    {formatBytes(category.allocatedBytes)}
                  </td>
                  <td className="numeric">
                    {formatBytes(category.logicalBytes)}
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
      <Notice>
        Logical size differs from allocated storage. Hard links count once;
        directory aggregates are not added again. APFS clone sharing, snapshots,
        filesystem overhead and unscanned data prevent physical reconciliation
        or a reclaim estimate. Values capped at unsigned 64-bit maximum may be
        lower bounds.
      </Notice>
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
      <p className="scan-phase" role="status">
        {phase}
      </p>
      {snapshot && (
        <p className="secondary">
          {elapsed}s elapsed
          {running && snapshot.progress.currentLocation
            ? ` · ${snapshot.progress.currentLocation}`
            : ''}
        </p>
      )}
      {running && (
        <p className="secondary">
          Current root shown above. Total work unknown; no percent estimate.
        </p>
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
