import { Folder } from '@phosphor-icons/react/Folder';
import { FolderOpen } from '@phosphor-icons/react/FolderOpen';
import { File } from '@phosphor-icons/react/File';
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import type { KeyboardEvent, ReactElement } from 'react';
import { useScanSession } from '../scan/useScanSession';
import type { ScanSnapshot } from '../scan/session';
import { formatBytes } from '../scan/accounting';
import { EmptyState, Notice, ResultTable, Toolbar } from '../shell/primitives';
import { ItemInspector } from './ItemInspector';
import { Treemap } from './Treemap';
import {
  foldedLabel,
  hasFoldedItems,
  measurementStatus,
  pageSize,
  searchDebounceMs,
  sortDirections,
  sortLabels,
} from './model';
import type { SortKey } from './model';
import { useListing } from './useListing';
import type { ListingSource } from './useListing';
import { useResolvedFindings } from './useResolvedFindings';
import type { Finding } from '../scan/contract';
import type { InspectionApi } from './contract';

interface ViewState {
  query: string;
  sort: SortKey;
  page: number;
  scroll: number;
}
const initialView: ViewState = {
  query: '',
  sort: 'size',
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
function useDebounced(value: string, delay: number): string {
  const [debounced, setDebounced] = useState(value);
  useEffect(() => {
    const timer = setTimeout(() => setDebounced(value), delay);
    return () => clearTimeout(timer);
  }, [value, delay]);
  return debounced;
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
  const generation = snapshot.progress.generation;
  const [trails, setTrails] = useState<Map<string, Finding[]>>(() => new Map());
  const [selections, setSelections] = useState<Map<string, string | null>>(
    () => new Map(),
  );
  const [selected, setSelected] = useState<Map<string, Finding>>(
    () => new Map(),
  );
  const [notice, setNotice] = useState('');
  const [quietSelection, setQuietSelection] = useState(false);
  const [layout, setLayout] = useState<'list' | 'treemap'>('list');
  const [views, setViews] = useState<Map<string, ViewState>>(() => new Map());
  const region = useRef<HTMLDivElement>(null);
  const searchReference = useRef<HTMLInputElement>(null);
  const investigation =
    state.investigation?.generation === generation ? state.investigation : null;
  const investigationKey = investigation?.category ?? 'browse';
  const selectedId =
    investigation && 'selectedId' in investigation
      ? (investigation.selectedId ?? null)
      : (selections.get(investigationKey) ?? null);
  function select(findingId: string | null, finding?: Finding): void {
    if (finding)
      setSelected((current) => new Map(current).set(finding.id, finding));
    if (investigation && 'selectedId' in investigation) {
      session.investigate({ ...investigation, selectedId: findingId });
      return;
    }
    setSelections((current) =>
      new Map(current).set(investigationKey, findingId),
    );
  }
  const trail = useMemo(
    () => trails.get(investigationKey) ?? [],
    [trails, investigationKey],
  );
  const directory = trail[trail.length - 1] ?? null;
  const directoryId = directory?.id ?? null;
  const scopeKey = `${investigation?.category ?? ''}:${directoryId ?? 'roots'}`;
  const view = views.get(scopeKey) ?? initialView;
  const typed = view.query.trim();
  const query = useDebounced(typed, searchDebounceMs);
  const searching = typed.length > 0;
  const settled = snapshot.settled;
  const source = useMemo<ListingSource | null>(() => {
    if (!settled) return null;
    if (searching) return query === typed ? { kind: 'search', query } : null;
    if (investigation && directoryId === null)
      return {
        kind: 'findings',
        ids: investigation.findingIds,
        sort: view.sort,
      };
    return { kind: 'children', parentId: directoryId, sort: view.sort };
  }, [settled, searching, query, typed, investigation, directoryId, view.sort]);
  const scanApi = session.api;
  const { listing, loading, failed, loadMore, retry } = useListing(
    scanApi,
    generation,
    source,
  );
  const waitingForQuery = searching && source === null;
  const rows = listing?.rows ?? [];
  const isSearch = source?.kind === 'search';
  const totalRows = isSearch ? rows.length : (listing?.total ?? 0);
  const pageCount = Math.max(1, Math.ceil(totalRows / pageSize));
  const page = Math.min(view.page, pageCount - 1);
  const visibleRows = rows.slice(page * pageSize, (page + 1) * pageSize);
  const onLastPage = page + 1 >= pageCount && !listing?.nextCursor;
  const folded =
    !isSearch && onLastPage && hasFoldedItems(listing?.folded ?? null)
      ? (listing?.folded ?? null)
      : null;
  const resolvedSelection = useResolvedFindings(
    scanApi,
    generation,
    selectedId && !selected.has(selectedId) ? [selectedId] : [],
    settled,
  );
  const selection = selectedId
    ? (selected.get(selectedId) ??
      resolvedSelection.get(selectedId) ??
      rows.find((row) => row.id === selectedId) ??
      null)
    : null;

  useEffect(() => {
    if (listing?.nextCursor && !loading && rows.length < (page + 1) * pageSize)
      loadMore();
  }, [listing, loading, rows.length, page, loadMore]);
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
  function setTrail(next: Finding[]): void {
    setTrails((current) => new Map(current).set(investigationKey, next));
    setNotice('');
  }
  function openFolder(finding: Finding): void {
    setTrail([...trail, finding]);
  }
  function goUp(): void {
    setTrail(trail.slice(0, -1));
  }
  function inspectRelated(findingId: string): void {
    setNotice(
      'Related item opened within the same scan; breadcrumb and filters retained.',
    );
    select(findingId);
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
      select(next.id, next);
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
      openFolder(selection);
      return;
    }
    if (event.key === 'ArrowLeft' && directoryId !== null) {
      event.preventDefault();
      goUp();
    }
  }
  function sortBy(sort: SortKey): void {
    editView({ sort, page: 0, scroll: 0 });
  }
  function sortHeader(sort: SortKey, numeric = false): ReactElement {
    return (
      <th
        scope="col"
        className={numeric ? 'numeric' : undefined}
        aria-sort={
          !isSearch && view.sort === sort ? sortDirections[sort] : 'none'
        }
      >
        <button disabled={isSearch} onClick={() => sortBy(sort)}>
          {sortLabels[sort]}
          {!isSearch && view.sort === sort
            ? sortDirections[sort] === 'descending'
              ? ' ↓'
              : ' ↑'
            : ''}
        </button>
      </th>
    );
  }
  function selectFinding(finding: Finding): void {
    setQuietSelection(false);
    select(finding.id, finding);
  }
  const directoryName = directory?.displayName ?? 'Scan locations';
  const foldedText = folded
    ? foldedLabel(folded, formatBytes(folded.allocatedBytes))
    : '';
  const summary = isSearch
    ? `${listing?.total ?? 0} listed matches · showing the largest ${rows.length}`
    : `${totalRows} listed entries`;
  return (
    <div className="explore-layout">
      <div className="explore-content" onKeyDown={handleBrowseKeys}>
        <nav aria-label="Folder breadcrumbs" className="breadcrumbs">
          <button
            onClick={() => setTrail([])}
            aria-current={directoryId === null ? 'location' : undefined}
          >
            Scan locations
          </button>
          {trail.map((finding, position) => (
            <button
              key={finding.id}
              onClick={() => setTrail(trail.slice(0, position + 1))}
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
              {summary} · hidden shown
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
          <button disabled={directoryId === null} onClick={goUp}>
            Up one folder
          </button>
          <label className="explore-search">
            Search listed items
            <input
              ref={searchReference}
              type="search"
              maxLength={256}
              value={view.query}
              onChange={(event) =>
                editView({ query: event.target.value, page: 0, scroll: 0 })
              }
              placeholder="Name of a listed item, including hidden files"
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
            {investigation.category} · filtered scan findings. Opening a folder
            browses it unfiltered; Reset filters restores all findings.
          </Notice>
        )}
        <p className="secondary explore-caveat">
          Allocated contribution · directory totals include descendants; shared
          identities count once. Not guaranteed recoverable space.{' '}
          {searching
            ? 'Searches listed items across the whole scan only; smaller items summarized into folders are not searched, so results are never a complete count. '
            : 'Smaller items inside a folder may be summarized in a "not listed" row. '}
          Hidden entries included. Arrow keys move the selection; Right opens a
          folder, Left goes up. Size does not imply recoverable space and age
          does not imply disuse.
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
            (failed
              ? ''
              : loading || waitingForQuery
                ? 'Loading listed items…'
                : `${summary} · page ${page + 1} of ${pageCount} · at most ${pageSize} rows rendered`)}
        </p>
        {failed && (
          <Notice>
            Listing unavailable for this scope. Retained measurements are
            unchanged. <button onClick={retry}>Retry</button>
          </Notice>
        )}
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
                {sortHeader('name')}
                {sortHeader('size', true)}
                <th scope="col">Type</th>
                {sortHeader('modified')}
                <th scope="col">Status</th>
                <th scope="col">Actions</th>
              </tr>
            </thead>
            <tbody>
              {visibleRows.map((finding) => (
                <tr
                  key={finding.id}
                  data-selected={selectedId === finding.id ? 'true' : undefined}
                  onClick={() => selectFinding(finding)}
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
                      {finding.parentId
                        ? `${listing?.parentNames[finding.parentId] ?? directory?.displayName ?? 'Parent'} · `
                        : ''}
                      {finding.id}
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
                    {finding.modifiedAtMs === null
                      ? 'Unknown'
                      : new Date(finding.modifiedAtMs).toLocaleDateString()}
                  </td>
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
                            openFolder(finding);
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
                          selectFinding(finding);
                        }}
                        aria-label={`Inspect ${finding.displayName}`}
                      >
                        Inspect
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
              {folded && (
                <tr className="folded-row" data-folded="true">
                  <th scope="row">{foldedText}</th>
                  <td className="numeric">
                    {formatBytes(folded.allocatedBytes)}
                  </td>
                  <td colSpan={4}>
                    Summarized: measured in the totals, not listed individually
                  </td>
                </tr>
              )}
            </tbody>
          </ResultTable>
        </div>
        {layout === 'treemap' && (
          <Treemap
            findings={visibleRows}
            folded={folded}
            selectedId={selectedId}
            onSelect={selectFinding}
            onOpen={openFolder}
          />
        )}
        {listing && rows.length === 0 && !folded && !loading && (
          <p>
            {isSearch
              ? 'No listed item matches. Smaller items summarized into folders are not searched. Reset filters or go up one folder.'
              : 'No entries in this scope. Reset filters or go up one folder.'}
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
          const wasListed = visibleRows.some(
            (finding) => finding.id === selectedId,
          );
          select(null);
          if (!wasListed) searchReference.current?.focus();
        }}
        onInspectRelated={inspectRelated}
        api={api}
      />
    </div>
  );
}
