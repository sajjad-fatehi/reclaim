import { Folder } from '@phosphor-icons/react/Folder';
import { FolderOpen } from '@phosphor-icons/react/FolderOpen';
import { File } from '@phosphor-icons/react/File';
import { useLayoutEffect, useMemo, useRef, useState } from 'react';
import type { KeyboardEvent, ReactElement } from 'react';
import { useScanSession } from '../scan/useScanSession';
import type { ScanSnapshot } from '../scan/session';
import { formatBytes } from '../scan/accounting';
import { EmptyState, Notice, ResultTable, Toolbar } from '../shell/primitives';
import { ItemInspector } from './ItemInspector';
import { Treemap } from './Treemap';
import {
  ancestors,
  browse,
  indexFindings,
  measurementStatus,
  pageSize,
} from './model';
import type { SortKey } from './model';
import type { InspectionApi } from './contract';

interface ViewState {
  query: string;
  sort: SortKey;
  descending: boolean;
  page: number;
  scroll: number;
}
const initialView: ViewState = {
  query: '',
  sort: 'size',
  descending: true,
  page: 0,
  scroll: 0,
};
export function Explore({
  api,
  onOverview,
  active = true,
}: {
  active?: boolean;
  api?: InspectionApi | undefined;
  onOverview: () => void;
}): ReactElement {
  const { state } = useScanSession();
  if (!state.snapshot)
    return (
      <EmptyState
        icon={<Folder size={28} />}
        title="Explore your measured files"
      >
        <p>
          Start a scan from Overview. Hidden and unrecognized files remain
          explorable within selected locations.
        </p>
        <button onClick={onOverview}>Go to Overview</button>
      </EmptyState>
    );
  return (
    <ExploreSnapshot
      key={state.snapshot.progress.generation}
      active={active}
      snapshot={state.snapshot}
      stale={state.stale}
      api={api}
      onOverview={onOverview}
    />
  );
}
function ExploreSnapshot({
  snapshot,
  active,
  stale,
  api,
  onOverview,
}: {
  snapshot: ScanSnapshot;
  active: boolean;
  stale: boolean;
  api?: InspectionApi | undefined;
  onOverview: () => void;
}): ReactElement {
  const { state, session } = useScanSession();
  const [directories, setDirectories] = useState<Map<string, string | null>>(
    () => new Map(),
  );
  const [selections, setSelections] = useState<Map<string, string | null>>(
    () => new Map(),
  );
  const [notice, setNotice] = useState('');
  const [quietSelection, setQuietSelection] = useState(false);
  const [layout, setLayout] = useState<'list' | 'treemap'>('list');
  const [views, setViews] = useState<Map<string, ViewState>>(() => new Map());
  const region = useRef<HTMLDivElement>(null);
  const searchReference = useRef<HTMLInputElement>(null);
  const index = useMemo(
    () => indexFindings(snapshot.findings),
    [snapshot.findings],
  );
  const investigation =
    state.investigation?.generation === snapshot.progress.generation
      ? state.investigation
      : null;
  const investigationKey = investigation?.category ?? 'browse';
  const selectedId =
    investigation && 'selectedId' in investigation
      ? (investigation.selectedId ?? null)
      : (selections.get(investigationKey) ?? null);
  function setSelectedId(findingId: string | null): void {
    if (investigation && 'selectedId' in investigation) {
      session.investigate({ ...investigation, selectedId: findingId });
      return;
    }
    setSelections((current) =>
      new Map(current).set(investigationKey, findingId),
    );
  }
  const directoryId = directories.get(investigationKey) ?? null;
  const findingIds = useMemo(
    () => (investigation ? new Set(investigation.findingIds) : null),
    [investigation],
  );
  const scopeKey = `${investigation?.category ?? ''}:${directoryId ?? 'roots'}`;
  const view = views.get(scopeKey) ?? initialView;
  const rows = useMemo(
    () =>
      browse(index, {
        directoryId,
        query: view.query,
        sort: view.sort,
        descending: view.descending,
        findingIds,
      }),
    [index, directoryId, view.query, view.sort, view.descending, findingIds],
  );
  const pageCount = Math.max(1, Math.ceil(rows.length / pageSize));
  const page = Math.min(view.page, pageCount - 1);
  const visibleRows = rows.slice(page * pageSize, (page + 1) * pageSize);
  const selection = selectedId ? (index.byId.get(selectedId) ?? null) : null;
  function editView(edits: Partial<ViewState>): void {
    setViews((current) =>
      new Map(current).set(scopeKey, {
        ...(current.get(scopeKey) ?? initialView),
        ...edits,
      }),
    );
  }
  useLayoutEffect(() => {
    const table = region.current?.querySelector('.table-scroll');
    if (table) table.scrollTop = views.get(scopeKey)?.scroll ?? 0;
  }, [scopeKey, views]);
  function navigate(findingId: string | null): void {
    setDirectories((current) =>
      new Map(current).set(investigationKey, findingId),
    );
    setNotice('');
  }
  function inspectRelated(findingId: string): void {
    if (!index.byId.has(findingId)) {
      setNotice(
        'Related item not loaded or outside this scan. Wait for measurements or scan again.',
      );
      return;
    }
    setSelectedId(findingId);
    setNotice(
      'Related item opened within the same scan; breadcrumb and filters retained.',
    );
  }
  function handleBrowseKeys(event: KeyboardEvent<HTMLElement>): void {
    const target = event.target;
    if (
      event.defaultPrevented ||
      event.altKey ||
      event.ctrlKey ||
      event.metaKey ||
      (target instanceof HTMLElement &&
        !target.closest('.table-scroll, .treemap')) ||
      (target instanceof HTMLElement && target.closest('th'))
    )
      return;
    const position = visibleRows.findIndex((row) => row.id === selectedId);
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      const step = event.key === 'ArrowDown' ? 1 : -1;
      const next = visibleRows[position === -1 ? 0 : position + step];
      if (!next) return;
      event.preventDefault();
      setQuietSelection(true);
      setSelectedId(next.id);
      Array.from(
        event.currentTarget.querySelectorAll<HTMLElement>(
          'button[data-finding-id]',
        ),
      )
        .find((button) => button.dataset.findingId === next.id)
        ?.focus();
      return;
    }
    if (event.key === 'ArrowRight' && selection?.kind === 'directory') {
      event.preventDefault();
      navigate(selection.id);
      return;
    }
    if (event.key === 'ArrowLeft' && directoryId !== null) {
      event.preventDefault();
      navigate(index.byId.get(directoryId)?.parentId ?? null);
    }
  }
  function sortBy(sort: SortKey): void {
    editView({
      sort,
      descending: view.sort === sort ? !view.descending : false,
      page: 0,
      scroll: 0,
    });
  }
  function sortHeader(sort: SortKey, label: string): ReactElement {
    return (
      <th
        scope="col"
        aria-sort={
          view.sort === sort
            ? view.descending
              ? 'descending'
              : 'ascending'
            : 'none'
        }
      >
        <button onClick={() => sortBy(sort)}>
          {label}
          {view.sort === sort ? (view.descending ? ' ↓' : ' ↑') : ''}
        </button>
      </th>
    );
  }
  function selectFinding(findingId: string): void {
    setQuietSelection(false);
    setSelectedId(findingId);
  }
  const directoryName = directoryId
    ? (index.byId.get(directoryId)?.displayName ?? 'Folder')
    : 'Scan locations';
  return (
    <div className="explore-layout">
      <div className="explore-content" onKeyDown={handleBrowseKeys}>
        <nav aria-label="Folder breadcrumbs" className="breadcrumbs">
          <button
            onClick={() => navigate(null)}
            aria-current={directoryId === null ? 'location' : undefined}
          >
            Scan locations
          </button>
          {ancestors(index, directoryId).map((finding) => (
            <button
              key={finding.id}
              onClick={() => navigate(finding.id)}
              aria-current={finding.id === directoryId ? 'location' : undefined}
              className="path-text"
            >
              {finding.displayName}
            </button>
          ))}
        </nav>
        <div className="explore-heading">
          <div>
            <h2 className="path-text">{directoryName}</h2>
            <p className="secondary">
              {rows.length} entries · hidden shown
              {layout === 'treemap' ? ' · tile area = allocated size' : ''}
            </p>
          </div>
          <div className="segmented" role="group" aria-label="Browsing layout">
            <button
              aria-pressed={layout === 'list'}
              onClick={() => setLayout('list')}
            >
              List
            </button>
            <button
              aria-pressed={layout === 'treemap'}
              onClick={() => setLayout('treemap')}
            >
              Treemap
            </button>
          </div>
        </div>
        <Toolbar label="Explore filters">
          <button
            disabled={directoryId === null}
            onClick={() =>
              navigate(
                directoryId
                  ? (index.byId.get(directoryId)?.parentId ?? null)
                  : null,
              )
            }
          >
            Up one folder
          </button>
          <label className="explore-search">
            Search within this scope
            <input
              ref={searchReference}
              type="search"
              value={view.query}
              onChange={(event) =>
                editView({ query: event.target.value, page: 0, scroll: 0 })
              }
              placeholder="Name or path, including hidden files"
            />
          </label>
          <button
            disabled={!view.query && !investigation}
            onClick={() => {
              editView({ query: '', page: 0, scroll: 0 });
              session.clearInvestigation();
            }}
          >
            Reset filters
          </button>
          <button onClick={onOverview}>Back to Overview</button>
        </Toolbar>
        {investigation && (
          <Notice>
            {investigation.category} · filtered scan findings. Opening folders
            retains this filter; Reset filters restores all findings.
          </Notice>
        )}
        <p className="secondary explore-caveat">
          Allocated contribution · directory totals include descendants; shared
          identities count once. Not guaranteed recoverable space. Search
          includes descendants; without filters, direct children shown. Hidden
          entries included. Arrow keys move the selection; Right opens a folder,
          Left goes up. Size does not imply recoverable space and age does not
          imply disuse.
        </p>
        {(stale ||
          !snapshot.settled ||
          snapshot.progress.status !== 'complete') && (
          <Notice>
            {stale ? 'Stale measurements. Scan again to refresh. ' : ''}Scan{' '}
            {snapshot.progress.status}
            {!snapshot.settled ? ' · measurements updating' : ''}. Partial,
            excluded, denied and vanished entries retain their measured state.
          </Notice>
        )}
        <p role="status" className="secondary">
          {notice ||
            `${rows.length} results · page ${page + 1} of ${pageCount} · at most ${pageSize} rows rendered`}
        </p>
        {selectedId && !selection && (
          <Notice>
            Selected item unavailable in this snapshot. Retained ID:{' '}
            {selectedId}. Scan again to refresh.
          </Notice>
        )}
        <div
          ref={region}
          hidden={layout !== 'list'}
          onScrollCapture={(event) => {
            if (
              event.target instanceof HTMLElement &&
              event.target.classList.contains('table-scroll')
            )
              editView({ scroll: event.target.scrollTop });
          }}
        >
          <ResultTable caption="Measured folders and files">
            <thead>
              <tr>
                {sortHeader('name', 'Name')}
                {sortHeader('size', 'Allocated size')}
                {sortHeader('kind', 'Type')}
                {sortHeader('status', 'Status')}
                <th scope="col">Actions</th>
              </tr>
            </thead>
            <tbody>
              {visibleRows.map((finding) => (
                <tr
                  key={finding.id}
                  data-selected={selectedId === finding.id ? 'true' : undefined}
                  onClick={() => selectFinding(finding.id)}
                >
                  <th scope="row">
                    <div className="finding-name">
                      {finding.kind === 'directory' ? (
                        <Folder size={20} aria-hidden="true" />
                      ) : (
                        <File size={20} aria-hidden="true" />
                      )}
                      <span className="path-text">{finding.displayName}</span>
                    </div>
                    <span className="secondary finding-path path-text">
                      {index.paths.get(finding.id)} · {finding.id}
                    </span>
                  </th>
                  <td className="numeric">
                    {formatBytes(finding.allocatedBytes)}
                    {!finding.complete && (
                      <span className="secondary"> · partial</span>
                    )}
                  </td>
                  <td>{finding.kind}</td>
                  <td>
                    <span
                      className="status-pill"
                      data-state={finding.issue ? 'issue' : undefined}
                    >
                      {measurementStatus(finding)}
                    </span>
                  </td>
                  <td>
                    <div className="row-actions">
                      {finding.kind === 'directory' && (
                        <button
                          onClick={(event) => {
                            event.stopPropagation();
                            navigate(finding.id);
                          }}
                          aria-label={`Open folder ${finding.displayName}`}
                        >
                          <FolderOpen size={16} aria-hidden="true" />
                          Open
                        </button>
                      )}
                      <button
                        data-finding-id={finding.id}
                        aria-pressed={selectedId === finding.id}
                        onClick={(event) => {
                          event.stopPropagation();
                          event.currentTarget.focus();
                          selectFinding(finding.id);
                        }}
                        aria-label={`Inspect ${finding.displayName}`}
                      >
                        Inspect
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </ResultTable>
        </div>
        {layout === 'treemap' && (
          <Treemap
            findings={visibleRows}
            selectedId={selectedId}
            onSelect={selectFinding}
            onOpen={navigate}
          />
        )}
        {rows.length === 0 && (
          <p>
            {snapshot.settled
              ? 'No matching entries in this scope. Reset filters or go up one folder.'
              : 'No matching entries yet. Measurements still loading.'}
          </p>
        )}
        <Toolbar label="Result pages">
          <button
            disabled={page === 0}
            onClick={() => editView({ page: page - 1, scroll: 0 })}
          >
            Previous page
          </button>
          <span>
            Page {page + 1} of {pageCount}
          </span>
          <button
            disabled={page + 1 >= pageCount}
            onClick={() => editView({ page: page + 1, scroll: 0 })}
          >
            Next page
          </button>
        </Toolbar>
      </div>
      <ItemInspector
        finding={active ? selection : null}
        progress={snapshot.progress}
        stale={stale}
        settled={snapshot.settled}
        focusOnChange={!quietSelection}
        onClose={() => {
          setSelectedId(null);
          if (!visibleRows.some((finding) => finding.id === selectedId))
            searchReference.current?.focus();
        }}
        onInspectRelated={inspectRelated}
        api={api}
      />
    </div>
  );
}
