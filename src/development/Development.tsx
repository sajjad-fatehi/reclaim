import { Code } from '@phosphor-icons/react/Code';
import { Wrench } from '@phosphor-icons/react/Wrench';
import { useEffect, useMemo, useRef, useState } from 'react';
import type { ReactElement } from 'react';
import { useScanSession } from '../scan/useScanSession';
import type { ScanSnapshot } from '../scan/session';
import { formatBytes } from '../scan/accounting';
import { EmptyState, Notice, ResultTable, Toolbar } from '../shell/primitives';
import { ItemInspector } from '../explore/ItemInspector';
import type { InspectionApi } from '../explore/contract';
import { locationOf, pageSize } from '../explore/model';
import { useResolvedFindings } from '../explore/useResolvedFindings';
import {
  familyLabels,
  natureLabels,
  dockerFailures,
  dockerErrorMessage,
  loadRecognition,
  nativeDevelopmentApi,
} from './contract';
import type {
  Artifact,
  DevelopmentApi,
  DockerSnapshot,
  RecognitionPage,
} from './contract';
import {
  artifactBytes,
  filterArtifacts,
  hasUncommittedWork,
  initialDevelopmentView,
  isProject,
  relatedWorktrees,
  worktreeState,
} from './model';
import type { DevelopmentTab, DevelopmentView } from './model';
import { ArtifactChip, CautionBadge, DockerChip } from './presentation';

interface DevelopmentProps {
  api?: DevelopmentApi | undefined;
  inspectionApi?: InspectionApi | undefined;
  active: boolean;
  onOverview: () => void;
  onExplore: (findingId: string) => void;
}
export function Development(props: DevelopmentProps): ReactElement {
  const { state } = useScanSession();
  if (state.loading) return <Notice>Loading shared scan session…</Notice>;
  if (!state.snapshot)
    return (
      <EmptyState icon={<Code size={28} />} title="Connect projects and tools">
        <p>
          Start a scan from Overview. Development uses recognized findings from
          your selected locations; missing artifacts do not prove a tool is
          uninstalled.
        </p>
        <button onClick={props.onOverview}>Go to Overview</button>
      </EmptyState>
    );
  return (
    <DevelopmentSnapshot
      key={state.snapshot.progress.generation}
      {...props}
      snapshot={state.snapshot}
      stale={state.stale}
    />
  );
}
function DevelopmentSnapshot({
  snapshot,
  stale,
  active,
  onOverview,
  onExplore,
  inspectionApi,
  api = nativeDevelopmentApi,
}: DevelopmentProps & {
  snapshot: ScanSnapshot;
  stale: boolean;
}): ReactElement {
  const { session } = useScanSession();
  const [report, setReport] = useState<RecognitionPage | null>(null);
  const [error, setError] = useState('');
  const [revision, setRevision] = useState(0);
  const [tab, setTab] = useState<DevelopmentTab>('projects');
  const [views, setViews] = useState<Record<DevelopmentTab, DevelopmentView>>({
    projects: { ...initialDevelopmentView },
    tools: { ...initialDevelopmentView },
  });
  const [docker, setDocker] = useState<DockerSnapshot | null>(null);
  const [dockerBusy, setDockerBusy] = useState(false);
  const [dockerMessage, setDockerMessage] = useState('');
  const [dockerSelection, setDockerSelection] = useState<string | null>(null);
  const [notice, setNotice] = useState('');
  const searchReference = useRef<HTMLInputElement>(null);
  const mounted = useRef(true);
  const dockerPending = useRef(false);
  const generation = snapshot.progress.generation;
  const settled = snapshot.settled;
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);
  useEffect(() => {
    if (!settled) return;
    let current = true;
    void loadRecognition(api, generation)
      .then((next) => {
        if (current) {
          setReport(next);
          setError('');
        }
      })
      .catch(() => {
        if (current)
          setError(
            'Recognition unavailable for this scan. Retained measurements remain inspectable in Explore. Retry recognition or scan again.',
          );
      });
    return () => {
      current = false;
    };
  }, [api, generation, settled, revision]);
  const view = views[tab];
  const artifacts = useMemo(() => report?.findings ?? [], [report]);
  const baseIds = useMemo(
    () => [
      ...new Set([
        ...artifacts.flatMap((artifact) =>
          artifact.projectId
            ? [artifact.findingId, artifact.projectId]
            : [artifact.findingId],
        ),
        ...(report?.diagnostics.map((diagnostic) => diagnostic.findingId) ??
          []),
        ...(view.selectedId ? [view.selectedId] : []),
      ]),
    ],
    [artifacts, report, view.selectedId],
  );
  const resolved = useResolvedFindings(
    session.api,
    generation,
    baseIds,
    settled,
    true,
  );
  const names = useMemo(
    () =>
      new Map(
        [...resolved.keys()].flatMap((id) => {
          const location = locationOf(resolved, id);
          return location ? [[id, location.path] as const] : [];
        }),
      ),
    [resolved],
  );
  const rows = filterArtifacts(artifacts, resolved, names, tab, view);
  const pageCount = Math.max(1, Math.ceil(rows.length / pageSize));
  const page = Math.min(view.page, pageCount - 1);
  const visibleRows = rows.slice(page * pageSize, (page + 1) * pageSize);
  const selected =
    artifacts.find((artifact) => artifact.findingId === view.selectedId) ??
    null;
  const selection = view.selectedId
    ? (resolved.get(view.selectedId) ?? null)
    : null;
  const inventory = docker && 'Ok' in docker.result ? docker.result.Ok : null;
  const selectedDocker =
    inventory?.items.find(
      (item) => item.artifact.findingId === dockerSelection,
    ) ?? null;
  function editView(edits: Partial<DevelopmentView>): void {
    setViews((current) => ({
      ...current,
      [tab]: { ...current[tab], ...edits },
    }));
  }
  function selectArtifact(findingId: string): void {
    if (
      !resolved.has(findingId) &&
      !artifacts.some((artifact) => artifact.findingId === findingId)
    ) {
      setNotice(
        'Related item outside this loaded scan. Scan its location to inspect it.',
      );
      return;
    }
    setNotice('');
    setDockerSelection(null);
    editView({ selectedId: findingId });
  }
  function showProject(findingId: string): void {
    setTab('projects');
    setDockerSelection(null);
    setViews((current) => ({
      ...current,
      projects: { ...current.projects, selectedId: findingId },
    }));
  }
  async function measureDocker(): Promise<void> {
    if (dockerPending.current) return;
    dockerPending.current = true;
    setDockerBusy(true);
    setDockerMessage('');
    try {
      const next = await api.dockerInventory();
      if (mounted.current) {
        setDocker(next);
        setDockerSelection(null);
      }
    } catch (error) {
      if (mounted.current)
        setDockerMessage(
          `${dockerErrorMessage(error)} Retained inventory is stale.`,
        );
    } finally {
      dockerPending.current = false;
      if (mounted.current) setDockerBusy(false);
    }
  }
  async function cancelDocker(): Promise<void> {
    try {
      await api.cancelDocker();
    } catch {
      if (mounted.current)
        setDockerMessage(
          'Cancellation request failed. Inventory may still finish; retry Cancel inventory.',
        );
    }
  }
  function name(artifact: Artifact): string {
    return (
      resolved.get(artifact.findingId)?.displayName ?? 'Loading item name…'
    );
  }
  function location(findingId: string): string {
    return names.get(findingId) ?? 'Resolving location…';
  }
  const projectCount = artifacts.filter(isProject).length;
  const uncommittedCount = artifacts.filter((artifact) =>
    hasUncommittedWork(artifact.worktree),
  ).length;
  const persistentCount = inventory
    ? inventory.items.filter((item) => item.persistent).length
    : 0;
  const ownedArtifacts = selected
    ? artifacts.filter(
        (artifact) =>
          artifact.findingId !== selected.findingId &&
          artifact.projectId === selected.findingId,
      )
    : [];
  const worktrees = selected ? relatedWorktrees(artifacts, selected) : [];
  return (
    <div className="explore-layout development-layout">
      <div className="explore-content">
        <div className="development-intro">
          <span className="development-intro-icon">
            <Code size={28} aria-hidden="true" />
          </span>
          <div>
            <h2>See what belongs where</h2>
            <p>
              Follow source, generated artifacts and shared tools within this
              scan.
            </p>
          </div>
          {report && (
            <dl className="development-stats" aria-label="Recognition summary">
              <div>
                <dt>Projects</dt>
                <dd>{projectCount}</dd>
              </div>
              <div>
                <dt>Tool artifacts</dt>
                <dd>{artifacts.length}</dd>
              </div>
            </dl>
          )}
        </div>
        {uncommittedCount > 0 && (
          <p className="development-alert" role="note">
            <CautionBadge kind="worktree">Uncommitted work</CautionBadge>
            {uncommittedCount === 1
              ? '1 worktree in this scan holds changes'
              : `${uncommittedCount} worktrees in this scan hold changes`}{' '}
            that exist nowhere else. Inspection only; Reclaim never offers these
            for cleanup.
          </p>
        )}
        <div
          role="tablist"
          aria-label="Development views"
          className="development-tabs"
        >
          {(['projects', 'tools'] as const).map((entry) => (
            <button
              key={entry}
              id={`development-${entry}`}
              role="tab"
              aria-selected={tab === entry}
              aria-controls="development-results"
              tabIndex={tab === entry ? 0 : -1}
              onClick={() => setTab(entry)}
              onKeyDown={(event) => {
                if (
                  !['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(
                    event.key,
                  )
                )
                  return;
                event.preventDefault();
                const next =
                  event.key === 'Home'
                    ? 'projects'
                    : event.key === 'End'
                      ? 'tools'
                      : tab === 'projects'
                        ? 'tools'
                        : 'projects';
                setTab(next);
                document.getElementById(`development-${next}`)?.focus();
              }}
            >
              {entry === 'projects' ? (
                <Code size={20} aria-hidden="true" />
              ) : (
                <Wrench size={20} aria-hidden="true" />
              )}
              {entry === 'projects' ? 'Projects' : 'Tools'}
            </button>
          ))}
        </div>
        <section
          id="development-results"
          role="tabpanel"
          aria-labelledby={`development-${tab}`}
          tabIndex={0}
          className="development-results"
        >
          <Toolbar label="Development filters">
            <label className="explore-search">
              Search selected locations
              <input
                ref={searchReference}
                type="search"
                value={view.query}
                onChange={(event) =>
                  editView({ query: event.target.value, page: 0 })
                }
                placeholder="Name, path or tool"
              />
            </label>
            <label>
              Tool family
              <select
                value={view.family}
                onChange={(event) => {
                  const family =
                    artifacts.find(
                      (artifact) => artifact.family === event.target.value,
                    )?.family ?? '';
                  editView({ family, page: 0 });
                }}
              >
                <option value="">All families</option>
                {Array.from(
                  new Set(artifacts.map((artifact) => artifact.family)),
                )
                  .sort()
                  .map((family) => (
                    <option key={family} value={family}>
                      {familyLabels[family]}
                    </option>
                  ))}
              </select>
            </label>
            <label>
              Artifact type
              <select
                value={view.nature}
                onChange={(event) => {
                  const nature =
                    artifacts.find(
                      (artifact) => artifact.nature === event.target.value,
                    )?.nature ?? '';
                  editView({ nature, page: 0 });
                }}
              >
                <option value="">All types</option>
                {Array.from(
                  new Set(artifacts.map((artifact) => artifact.nature)),
                )
                  .sort()
                  .map((nature) => (
                    <option key={nature} value={nature}>
                      {natureLabels[nature]}
                    </option>
                  ))}
              </select>
            </label>
            <button
              onClick={() =>
                editView({ query: '', family: '', nature: '', page: 0 })
              }
            >
              Reset Development filters
            </button>
            <button onClick={onOverview}>Scan locations</button>
          </Toolbar>
          <div className="development-notes">
            <p className="secondary path-text">
              Scope:{' '}
              {snapshot.scope.roots.map((root) => root.displayPath).join(' · ')}
              . {snapshot.scope.exclusions.length} exclusions. Measured{' '}
              {new Date(
                snapshot.progress.finishedAtMs ?? snapshot.progress.startedAtMs,
              ).toLocaleString()}
              .
            </p>
            <Notice>
              {tab === 'projects'
                ? 'Native owned project totals include source and owned artifacts, excluding nested projects. Source remainder and artifact rows are breakdowns, not additional totals.'
                : 'Grouped by native tool family. Each exclusive allocated contribution excludes separately recognized descendants and counts shared file identities once. Project rows show source remainder only.'}{' '}
              Allocated bytes are not guaranteed recoverable space. Shared
              stores are never added to each related project.
            </Notice>
            {(stale || !settled || snapshot.progress.status !== 'complete') && (
              <Notice>
                {stale ? 'Stale scan. ' : ''}Scan {snapshot.progress.status}
                {!settled ? ' · measurements / recognition pending' : ''}.
                Denied, excluded and vanished locations limit coverage.{' '}
                {snapshot.progress.terminalIssue ?? ''}
              </Notice>
            )}
            {report &&
              (!report.finished ||
                report.limited ||
                report.cancelled ||
                report.diagnostics.length > 0) && (
                <Notice>
                  Recognition{' '}
                  {report.cancelled
                    ? 'cancelled'
                    : report.limited
                      ? 'limited'
                      : report.finished
                        ? 'finished with coverage limits'
                        : 'incomplete'}
                  . Unknown ownership remains unknown; no tool installation or
                  inactivity is inferred.
                </Notice>
              )}
          </div>
          {error && (
            <p role="alert">
              {error}{' '}
              <button
                onClick={() => {
                  setError('');
                  setRevision((current) => current + 1);
                }}
              >
                Retry recognition
              </button>
            </p>
          )}
          {!report && !error && (
            <p role="status">
              {settled
                ? 'Loading native recognition…'
                : 'Waiting for this scan to settle…'}
            </p>
          )}
          {!!report?.diagnostics.length && (
            <details>
              <summary>
                Recognition limitations ({report.diagnostics.length}
                {report.diagnosticsTruncated ? '+' : ''})
              </summary>
              <ul>
                {report.diagnostics.map((diagnostic, diagnosticIndex) => (
                  <li
                    key={`${diagnostic.findingId}:${diagnosticIndex}`}
                    className="path-text"
                  >
                    {diagnostic.reason.replaceAll('_', ' ')} ·{' '}
                    {location(diagnostic.findingId)}
                    <button
                      onClick={() => selectArtifact(diagnostic.findingId)}
                    >
                      Inspect affected item
                    </button>
                  </li>
                ))}
              </ul>
            </details>
          )}
          <p role="status">
            {notice ||
              `${rows.length} ${tab} findings · page ${page + 1} of ${pageCount}`}
          </p>
          <ResultTable
            caption={
              tab === 'projects'
                ? 'Recognized projects and worktrees'
                : 'Filesystem artifacts by tool family'
            }
          >
            <thead>
              <tr>
                <th
                  scope="col"
                  aria-sort={
                    view.sort === 'name'
                      ? view.descending
                        ? 'descending'
                        : 'ascending'
                      : 'none'
                  }
                >
                  <button
                    onClick={() =>
                      editView({
                        sort: 'name',
                        descending:
                          view.sort === 'name' ? !view.descending : false,
                        page: 0,
                      })
                    }
                  >
                    Name{' '}
                    {view.sort === 'name' ? (view.descending ? '↓' : '↑') : ''}
                  </button>
                </th>
                <th scope="col">Tool / type</th>
                <th
                  scope="col"
                  aria-sort={
                    view.sort === 'size'
                      ? view.descending
                        ? 'descending'
                        : 'ascending'
                      : 'none'
                  }
                >
                  <button
                    onClick={() =>
                      editView({
                        sort: 'size',
                        descending:
                          view.sort === 'size' ? !view.descending : true,
                        page: 0,
                      })
                    }
                  >
                    Allocated{' '}
                    {tab === 'projects'
                      ? 'owned total'
                      : 'exclusive contribution'}{' '}
                    {view.sort === 'size' ? (view.descending ? '↓' : '↑') : ''}
                  </button>
                </th>
                <th scope="col">State / actions</th>
              </tr>
            </thead>
            {(tab === 'tools'
              ? Object.keys(familyLabels).filter((family) =>
                  visibleRows.some((artifact) => artifact.family === family),
                )
              : ['projects']
            ).map((group) => (
              <ArtifactGroup
                key={group}
                group={group}
                tab={tab}
                rows={visibleRows}
                selectedId={view.selectedId}
                name={name}
                location={location}
                onInspect={selectArtifact}
                onExplore={onExplore}
                onProject={showProject}
              />
            ))}
          </ResultTable>
          {report && rows.length === 0 && (
            <p>
              No matching {tab} artifacts in selected locations. Reset filters
              or scan additional locations. This does not prove tools are
              missing; denied or excluded locations may hide artifacts.{' '}
              <button
                onClick={() => {
                  editView({ selectedId: null });
                  onExplore(
                    snapshot.scope.roots.length
                      ? (snapshot.findings[0]?.id ?? '')
                      : '',
                  );
                }}
              >
                Inspect unrecognized files in Explore
              </button>
            </p>
          )}
          <Toolbar label="Development result pages">
            <button
              disabled={page === 0}
              onClick={() => editView({ page: page - 1 })}
            >
              Previous Development page
            </button>
            <span>
              Page {page + 1} of {pageCount} · at most {pageSize} findings
              rendered
            </span>
            <button
              disabled={page + 1 >= pageCount}
              onClick={() => editView({ page: page + 1 })}
            >
              Next Development page
            </button>
          </Toolbar>
          {selected && (
            <section
              className="development-detail"
              aria-label="Selected artifact relationships"
            >
              <h3 className="path-text">{name(selected)}</h3>
              {hasUncommittedWork(selected.worktree) && (
                <CautionBadge kind="worktree">
                  Uncommitted changes would be lost
                </CautionBadge>
              )}
              <p>{worktreeState(selected.worktree)}</p>
              <p>
                Inspection only. Clean, unlocked or old does not mean removable.
                Preserve source, ignored/untracked data, unsaved work and
                persistent data.
              </p>
              <dl className="inspection-facts">
                <dt>Logical / allocated exclusive contribution</dt>
                <dd>
                  {formatBytes(selected.exclusiveLogicalBytes)} /{' '}
                  {formatBytes(selected.exclusiveAllocatedBytes)}
                </dd>
                {isProject(selected) && (
                  <>
                    <dt>Logical / allocated owned project total</dt>
                    <dd>
                      {formatBytes(selected.ownedLogicalBytes)} /{' '}
                      {formatBytes(selected.ownedAllocatedBytes)}
                    </dd>
                  </>
                )}
              </dl>
              {isProject(selected) && (
                <>
                  <h3>Source and owned artifacts</h3>
                  <p>
                    Source / configuration remainder:{' '}
                    {formatBytes(selected.exclusiveAllocatedBytes)} allocated.
                    Unrecognized content may remain in this remainder.
                  </p>
                  <ul>
                    {ownedArtifacts.map((artifact) => (
                      <li key={artifact.findingId}>
                        {natureLabels[artifact.nature]} · {name(artifact)} ·{' '}
                        {formatBytes(artifact.exclusiveAllocatedBytes)}{' '}
                        <button
                          onClick={() => selectArtifact(artifact.findingId)}
                        >
                          Inspect {name(artifact)}
                        </button>
                        <button
                          onClick={() => {
                            setTab('tools');
                            setViews((current) => ({
                              ...current,
                              tools: {
                                ...current.tools,
                                selectedId: artifact.findingId,
                                family: artifact.family,
                                query: '',
                                nature: '',
                                page: 0,
                              },
                            }));
                          }}
                        >
                          View in Tools
                        </button>
                      </li>
                    ))}
                  </ul>
                  {!ownedArtifacts.length && (
                    <p>
                      No separately recognized owned artifacts in this scan.
                    </p>
                  )}
                  <h3>Linked worktrees</h3>
                  <ul>
                    {worktrees.map((artifact) => (
                      <li key={artifact.findingId}>
                        {name(artifact)} · {worktreeState(artifact.worktree)}{' '}
                        <button onClick={() => showProject(artifact.findingId)}>
                          View related worktree
                        </button>
                      </li>
                    ))}
                  </ul>
                  {!worktrees.length && (
                    <p>
                      No linked worktrees verified within selected scope.
                      Outside-scope metadata stays unknown.
                    </p>
                  )}
                </>
              )}
              <h3>Related project</h3>
              {selected.projectId &&
              selected.projectId !== selected.findingId ? (
                <button
                  onClick={() =>
                    showProject(selected.projectId ?? selected.findingId)
                  }
                >
                  View related project
                </button>
              ) : (
                <p>
                  {isProject(selected)
                    ? 'This finding is a project root.'
                    : 'Shared or unassigned. Related projects unknown; a matching tool family does not prove use of this store.'}
                </p>
              )}
            </section>
          )}
          {tab === 'tools' && (
            <section
              className="development-detail"
              aria-label="Local Docker inventory"
            >
              <h3>Local Docker inventory</h3>
              <p>
                Separate observation; never added to filesystem totals.
                Inspection only. Reclaim does not install Docker, start its
                daemon or remove resources.
              </p>
              <Toolbar label="Docker inventory controls">
                <button
                  disabled={dockerBusy}
                  onClick={() => void measureDocker()}
                >
                  {docker
                    ? 'Refresh Docker inventory'
                    : 'Inspect Docker inventory'}
                </button>
                {dockerBusy && (
                  <button onClick={() => void cancelDocker()}>
                    Cancel inventory
                  </button>
                )}
              </Toolbar>
              <p role="status">
                {dockerBusy
                  ? 'Reading local Docker inventory… Retained observations may be stale.'
                  : dockerMessage ||
                    (docker
                      ? `Observed ${new Date(Number(docker.measuredAtMs)).toLocaleString()}; refresh to check current state.`
                      : 'Not inspected. Docker availability unknown.')}
              </p>
              {docker && 'Err' in docker.result && (
                <Notice>{dockerFailures[docker.result.Err]}</Notice>
              )}
              {inventory && (
                <>
                  {persistentCount > 0 && (
                    <p className="development-alert" role="note">
                      <CautionBadge kind="persistent">
                        Persistent container data
                      </CautionBadge>
                      {persistentCount} observed resource
                      {persistentCount === 1 ? '' : 's'} may hold databases or
                      files that exist nowhere else. Inspection only.
                    </p>
                  )}
                  <p>{inventory.accounting}</p>
                  {inventory.items.length === 0 ? (
                    <p>No Docker resources reported by this inventory.</p>
                  ) : (
                    <DockerRows
                      snapshot={docker}
                      query={view.query}
                      onInspect={(findingId) => {
                        editView({ selectedId: null });
                        setDockerSelection(findingId);
                      }}
                    />
                  )}
                </>
              )}
            </section>
          )}
        </section>
      </div>
      <ItemInspector
        finding={active ? selection : null}
        dockerItem={active && tab === 'tools' ? selectedDocker : null}
        dockerMeasuredAtMs={docker?.measuredAtMs}
        progress={snapshot.progress}
        stale={stale}
        settled={settled}
        onClose={() => {
          editView({ selectedId: null });
          setDockerSelection(null);
          searchReference.current?.focus();
        }}
        onInspectRelated={selectArtifact}
        api={inspectionApi}
      />
    </div>
  );
}
function ArtifactGroup({
  group,
  tab,
  rows,
  selectedId,
  name,
  location,
  onInspect,
  onExplore,
  onProject,
}: {
  group: string;
  tab: DevelopmentTab;
  rows: Artifact[];
  selectedId: string | null;
  name: (artifact: Artifact) => string;
  location: (findingId: string) => string;
  onInspect: (findingId: string) => void;
  onExplore: (findingId: string) => void;
  onProject: (findingId: string) => void;
}): ReactElement {
  const entries =
    tab === 'projects'
      ? rows
      : rows.filter((artifact) => artifact.family === group);
  return (
    <tbody>
      {tab === 'tools' && entries[0] && (
        <tr className="development-group">
          <th scope="rowgroup" colSpan={4}>
            {familyLabels[entries[0].family]}
          </th>
        </tr>
      )}
      {entries.map((artifact) => (
        <tr
          key={artifact.findingId}
          data-selected={selectedId === artifact.findingId ? 'true' : undefined}
        >
          <th scope="row">
            <div className="finding-name">
              <ArtifactChip artifact={artifact} />
              <div>
                <span className="path-text">{name(artifact)}</span>
                <span className="secondary finding-path path-text">
                  {location(artifact.findingId)}
                </span>
              </div>
            </div>
          </th>
          <td>
            {familyLabels[artifact.family]}
            <br />
            {natureLabels[artifact.nature]}
          </td>
          <td className="numeric">
            {formatBytes(artifactBytes(artifact, tab))}
            <br />
            <span className="secondary">
              {artifact.complete ? 'measured' : 'partial'}
            </span>
          </td>
          <td>
            {hasUncommittedWork(artifact.worktree) && (
              <CautionBadge kind="worktree">Uncommitted changes</CautionBadge>
            )}
            <p>
              {artifact.worktree
                ? worktreeState(artifact.worktree)
                : `${artifact.confidence} · inspection only`}
            </p>
            <div className="row-actions">
              <button
                aria-label={`Inspect ${name(artifact)}`}
                aria-pressed={selectedId === artifact.findingId}
                onClick={(event) => {
                  event.currentTarget.focus();
                  onInspect(artifact.findingId);
                }}
              >
                Inspect
              </button>
              <button
                aria-label={`Explore ${name(artifact)}`}
                onClick={() => onExplore(artifact.findingId)}
              >
                Explore
              </button>
              {artifact.projectId &&
                artifact.projectId !== artifact.findingId && (
                  <button
                    onClick={() =>
                      onProject(artifact.projectId ?? artifact.findingId)
                    }
                  >
                    Project
                  </button>
                )}
            </div>
          </td>
        </tr>
      ))}
    </tbody>
  );
}
function DockerRows({
  snapshot,
  query,
  onInspect,
}: {
  snapshot: DockerSnapshot | null;
  query: string;
  onInspect: (identity: string) => void;
}): ReactElement {
  const inventory =
    snapshot && 'Ok' in snapshot.result ? snapshot.result.Ok : null;
  const rows =
    inventory?.items.filter((item) =>
      `${item.identity} ${item.kind}`
        .toLocaleLowerCase()
        .includes(query.toLocaleLowerCase()),
    ) ?? [];
  const [page, setPage] = useState(0);
  const count = Math.max(1, Math.ceil(rows.length / pageSize));
  const current = Math.min(page, count - 1);
  return (
    <>
      <ResultTable caption="Docker observations, not additive totals">
        <thead>
          <tr>
            <th scope="col">Resource / identity</th>
            <th scope="col">Logical observation</th>
            <th scope="col">Protection / action</th>
          </tr>
        </thead>
        <tbody>
          {rows
            .slice(current * pageSize, (current + 1) * pageSize)
            .map((item) => (
              <tr key={item.artifact.findingId}>
                <th scope="row">
                  <div className="finding-name">
                    <DockerChip />
                    <div className="path-text">
                      {item.kind.replaceAll('_', ' ')}
                      <span className="finding-path">{item.identity}</span>
                    </div>
                  </div>
                </th>
                <td>{formatBytes(item.logicalBytes)}</td>
                <td>
                  {item.persistent && (
                    <CautionBadge kind="persistent">
                      Persistent data
                    </CautionBadge>
                  )}
                  {item.active === null
                    ? 'Activity unknown'
                    : item.active
                      ? 'Active'
                      : 'Not active at observation'}
                  <br />
                  <button
                    aria-label={`Inspect Docker ${item.kind} ${item.identity}`}
                    onClick={(event) => {
                      event.currentTarget.focus();
                      onInspect(item.artifact.findingId);
                    }}
                  >
                    Inspect
                  </button>
                </td>
              </tr>
            ))}
        </tbody>
      </ResultTable>
      {!rows.length && (
        <p>No matching Docker observations. Reset search to see inventory.</p>
      )}
      <Toolbar label="Docker result pages">
        <button disabled={current === 0} onClick={() => setPage(current - 1)}>
          Previous Docker page
        </button>
        <span>
          Page {current + 1} of {count}
        </span>
        <button
          disabled={current + 1 >= count}
          onClick={() => setPage(current + 1)}
        >
          Next Docker page
        </button>
      </Toolbar>
    </>
  );
}
