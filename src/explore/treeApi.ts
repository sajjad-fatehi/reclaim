import type { ChildSort, Finding, ScanApi } from '../scan/contract';
import { sortFindings } from './model';

/**
 * Serves `scan_children`, `scan_search` and `scan_findings` from an in-memory
 * finding tree for fixtures and tests. `folded` maps a directory id to the
 * summarized remainder native retention would report for it.
 */
export function createTreeApi(
  base: ScanApi,
  findings: Finding[],
  folded: Record<string, { items: number; allocatedBytes: string }> = {},
): ScanApi {
  const chunk = 200;
  return {
    ...base,
    children: async (generation, parentId, cursor, sort: ChildSort) => {
      const siblings = sortFindings(
        findings.filter((finding) => finding.parentId === parentId),
        sort,
      );
      const start = cursor ? Number(cursor) : 0;
      const end = start + chunk;
      const remainder = parentId ? folded[parentId] : undefined;
      return {
        generation,
        parentId,
        sort,
        total: siblings.length,
        nextCursor: end < siblings.length ? String(end) : null,
        findings: siblings.slice(start, end),
        folded: remainder
          ? {
              items: remainder.items,
              logicalBytes: remainder.allocatedBytes,
              allocatedBytes: remainder.allocatedBytes,
            }
          : null,
      };
    },
    search: async (generation, query) => {
      const needle = query.toLocaleLowerCase();
      const matches = sortFindings(
        findings.filter(
          (finding) =>
            finding.parentId !== null &&
            finding.displayName.toLocaleLowerCase().includes(needle),
        ),
        'size',
      );
      return {
        generation,
        total: matches.length,
        findings: matches.slice(0, chunk),
      };
    },
    findings: async (_generation, ids) =>
      ids.map((id) => {
        const finding = findings.find((candidate) => candidate.id === id);
        if (!finding) throw { code: 'invalid_request', issue: null };
        return finding;
      }),
  };
}

export function pathOf(findings: Finding[], findingId: string): string {
  const byId = new Map(findings.map((finding) => [finding.id, finding]));
  const names: string[] = [];
  const visited = new Set<string>();
  let current = byId.get(findingId);
  while (current && !visited.has(current.id)) {
    visited.add(current.id);
    names.push(current.displayName);
    current = current.parentId ? byId.get(current.parentId) : undefined;
  }
  return names.reverse().join('/').replace(/^\/\//, '/');
}
