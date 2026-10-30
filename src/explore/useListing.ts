import { useCallback, useEffect, useRef, useState } from 'react';
import type { ChildSort, Finding, FoldedRow, ScanApi } from '../scan/contract';
import { readThrottled } from './reads';
import { searchLimit, sortFindings } from './model';

export type ListingSource =
  | { kind: 'children'; parentId: string | null; sort: ChildSort }
  | { kind: 'search'; query: string }
  | { kind: 'findings'; ids: string[]; sort: ChildSort };

export interface Listing {
  rows: Finding[];
  total: number;
  folded: FoldedRow | null;
  nextCursor: string | null;
  parentNames: Record<string, string>;
}
export interface ListingState {
  listing: Listing | null;
  loading: boolean;
  failed: boolean;
  loadMore: () => void;
  retry: () => void;
}

const batchSize = 200;

export function sourceKey(source: ListingSource): string {
  switch (source.kind) {
    case 'children':
      return `children:${source.parentId ?? 'roots'}:${source.sort}`;
    case 'search':
      return `search:${source.query}`;
    default:
      return `findings:${source.sort}:${source.ids.join(',')}`;
  }
}

export async function resolveFindings(
  api: ScanApi,
  generation: number,
  ids: string[],
  isCurrent: () => boolean,
): Promise<Finding[]> {
  const resolved: Finding[] = [];
  for (let start = 0; start < ids.length; start += batchSize) {
    const batch = ids.slice(start, start + batchSize);
    resolved.push(
      ...(await readThrottled(
        api,
        () => api.findings(generation, batch),
        isCurrent,
      )),
    );
  }
  return resolved;
}

async function loadFirst(
  api: ScanApi,
  generation: number,
  source: ListingSource,
  isCurrent: () => boolean,
): Promise<Listing> {
  switch (source.kind) {
    case 'children': {
      const page = await readThrottled(
        api,
        () => api.children(generation, source.parentId, null, source.sort),
        isCurrent,
      );
      return {
        rows: page.findings,
        total: page.total,
        folded: page.folded,
        nextCursor: page.nextCursor,
        parentNames: {},
      };
    }
    case 'search': {
      const result = await readThrottled(
        api,
        () => api.search(generation, source.query),
        isCurrent,
      );
      const parentIds = [
        ...new Set(
          result.findings.flatMap((finding) =>
            finding.parentId ? [finding.parentId] : [],
          ),
        ),
      ].slice(0, searchLimit);
      const parents = await resolveFindings(
        api,
        generation,
        parentIds,
        isCurrent,
      ).catch(() => []);
      return {
        rows: result.findings,
        total: result.total,
        folded: null,
        nextCursor: null,
        parentNames: Object.fromEntries(
          parents.map((parent) => [parent.id, parent.displayName]),
        ),
      };
    }
    default: {
      const rows = sortFindings(
        await resolveFindings(api, generation, source.ids, isCurrent),
        source.sort,
      );
      return {
        rows,
        total: rows.length,
        folded: null,
        nextCursor: null,
        parentNames: {},
      };
    }
  }
}

/**
 * Loads one scope of the immutable native snapshot. Listings are cached per
 * key for the lifetime of the generation, responses for a scope the user has
 * left are dropped, and `rate_limited` is retried inside `readThrottled`.
 */
export function useListing(
  api: ScanApi,
  generation: number,
  source: ListingSource | null,
): ListingState {
  const [cache, setCache] = useState(() => new Map<string, Listing>());
  const requested = useRef(new Set<string>());
  const current = useRef<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [failed, setFailed] = useState(false);
  const key = source ? sourceKey(source) : null;
  const listing = key ? (cache.get(key) ?? null) : null;

  useEffect(() => {
    current.current = key;
  }, [key]);

  const fetchFirst = useCallback(
    (scope: ListingSource, scopeKey: string): void => {
      const isCurrent = (): boolean => current.current === scopeKey;
      requested.current.add(scopeKey);
      setLoading(true);
      setFailed(false);
      loadFirst(api, generation, scope, isCurrent)
        .then((loaded) => {
          setCache((existing) => new Map(existing).set(scopeKey, loaded));
        })
        .catch(() => {
          requested.current.delete(scopeKey);
          if (isCurrent()) setFailed(true);
        })
        .finally(() => {
          if (isCurrent()) setLoading(false);
        });
    },
    [api, generation],
  );

  useEffect(() => {
    if (!source || !key || requested.current.has(key)) return;
    fetchFirst(source, key);
    // `source` is represented by `key`; rebuilding it must not refetch.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, fetchFirst]);

  const loadMore = useCallback((): void => {
    if (!source || source.kind !== 'children' || !key || !listing?.nextCursor)
      return;
    const cursor = listing.nextCursor;
    const isCurrent = (): boolean => current.current === key;
    setLoading(true);
    readThrottled(
      api,
      () => api.children(generation, source.parentId, cursor, source.sort),
      isCurrent,
    )
      .then((page) => {
        setCache((existing) => {
          const latest = existing.get(key);
          if (!latest || latest.nextCursor !== cursor) return existing;
          return new Map(existing).set(key, {
            ...latest,
            rows: [...latest.rows, ...page.findings],
            nextCursor: page.nextCursor,
          });
        });
      })
      .catch(() => {
        if (isCurrent()) setFailed(true);
      })
      .finally(() => {
        if (isCurrent()) setLoading(false);
      });
  }, [api, generation, key, source, listing]);

  const retry = useCallback((): void => {
    if (!source || !key) return;
    fetchFirst(source, key);
  }, [source, key, fetchFirst]);

  return { listing, loading, failed, loadMore, retry };
}
