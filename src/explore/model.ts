import type { Finding } from '../scan/contract';

export const pageSize = 50;
export type SortKey = 'name' | 'size' | 'kind' | 'status';
export interface BrowseOptions {
  directoryId: string | null;
  query: string;
  sort: SortKey;
  descending: boolean;
  findingIds: ReadonlySet<string> | null;
}
export interface FindingIndex {
  byId: Map<string, Finding>;
  paths: Map<string, string>;
  children: Map<string | null, Finding[]>;
}
export function indexFindings(findings: Finding[]): FindingIndex {
  const byId = new Map(findings.map((finding) => [finding.id, finding]));
  const paths = new Map<string, string>();
  const children = new Map<string | null, Finding[]>();
  for (const finding of findings) {
    const siblings = children.get(finding.parentId) ?? [];
    siblings.push(finding);
    children.set(finding.parentId, siblings);
    const chain: string[] = [];
    let current: Finding | undefined = finding;
    const visited = new Set<string>();
    while (current && !visited.has(current.id)) {
      visited.add(current.id);
      chain.push(current.displayName);
      current = current.parentId ? byId.get(current.parentId) : undefined;
    }
    paths.set(finding.id, chain.reverse().join('/').replace(/^\/\//, '/'));
  }
  return { byId, paths, children };
}
export function ancestors(
  index: FindingIndex,
  findingId: string | null,
): Finding[] {
  const chain: Finding[] = [];
  const visited = new Set<string>();
  let current = findingId ? index.byId.get(findingId) : undefined;
  while (current && !visited.has(current.id)) {
    visited.add(current.id);
    chain.push(current);
    current = current.parentId ? index.byId.get(current.parentId) : undefined;
  }
  return chain.reverse();
}
export function measurementStatus(finding: Finding): string {
  if (finding.issue) return finding.issue.replaceAll('_', ' ');
  if (finding.aliasOf) return 'Shared identity · counted once';
  return finding.complete ? 'Measured' : 'Partial measurement';
}
export function browse(index: FindingIndex, options: BrowseOptions): Finding[] {
  const query = options.query.trim().toLocaleLowerCase();
  const recursive = query.length > 0 || options.findingIds !== null;
  const candidates: Finding[] = [];
  const pending = [...(index.children.get(options.directoryId) ?? [])];
  const visited = new Set<string>();
  while (pending.length > 0) {
    const finding = pending.pop();
    if (!finding || visited.has(finding.id)) continue;
    visited.add(finding.id);
    if (recursive && finding.kind === 'directory') {
      for (const child of index.children.get(finding.id) ?? [])
        pending.push(child);
    }
    if (options.findingIds && !options.findingIds.has(finding.id)) continue;
    if (
      query &&
      !(index.paths.get(finding.id) ?? finding.displayName)
        .toLocaleLowerCase()
        .includes(query)
    )
      continue;
    candidates.push(finding);
  }
  return candidates.sort((left, right) => {
    let comparison: number;
    switch (options.sort) {
      case 'size': {
        const leftSize = BigInt(left.allocatedBytes);
        const rightSize = BigInt(right.allocatedBytes);
        comparison = leftSize < rightSize ? -1 : leftSize > rightSize ? 1 : 0;
        break;
      }
      case 'kind':
        comparison = left.kind.localeCompare(right.kind);
        break;
      case 'status':
        comparison = measurementStatus(left).localeCompare(
          measurementStatus(right),
        );
        break;
      default:
        comparison = left.displayName.localeCompare(right.displayName);
    }
    return (
      (options.descending ? -comparison : comparison) ||
      left.id.localeCompare(right.id)
    );
  });
}
