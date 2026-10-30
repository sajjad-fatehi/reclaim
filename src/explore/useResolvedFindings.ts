import { useEffect, useRef, useState } from 'react';
import type { Finding, ScanApi } from '../scan/contract';
import { locationOf } from './model';
import { resolveFindings } from './useListing';

/**
 * Resolves finding ids to findings through `scan_findings`, keeping what was already resolved.
 * With `followParents`, each id's nearest unresolved ancestor is fetched too, a bounded
 * number of levels (see `locationOf`), so rows can show a readable location.
 */
export function useResolvedFindings(
  api: ScanApi,
  generation: number,
  ids: readonly string[],
  enabled: boolean,
  followParents = false,
): Map<string, Finding> {
  const [resolved, setResolved] = useState(() => new Map<string, Finding>());
  const requested = useRef(new Set<string>());
  const signature = ids.join('\u0000');

  useEffect(() => {
    if (!enabled) return;
    const pending = requested.current;
    const wanted = new Set(ids);
    if (followParents)
      for (const id of ids) {
        const next = locationOf(resolved, id)?.missingId;
        if (next) wanted.add(next);
      }
    const missing = [...wanted].filter((id) => !pending.has(id));
    if (missing.length === 0) return;
    for (const id of missing) pending.add(id);
    let active = true;
    let completed = false;
    resolveFindings(api, generation, missing, () => active)
      .then((findings) => {
        completed = true;
        if (!active) return;
        setResolved((current) => {
          const next = new Map(current);
          for (const finding of findings) next.set(finding.id, finding);
          return next;
        });
      })
      .catch(() => {
        for (const id of missing) pending.delete(id);
      });
    return () => {
      active = false;
      if (completed) return;
      for (const id of missing) pending.delete(id);
    };
    // `ids` is represented by `signature`.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [api, generation, signature, enabled, followParents, resolved]);

  return resolved;
}
