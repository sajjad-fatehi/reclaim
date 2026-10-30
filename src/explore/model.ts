import type { ChildSort, Finding, FoldedRow } from '../scan/contract';

export const pageSize = 50;
export const searchDebounceMs = 250;
export const searchLimit = 200;
export type SortKey = ChildSort;
export const sortLabels: Record<SortKey, string> = {
  size: 'Allocated size',
  name: 'Name',
  modified: 'Modified',
};
export const sortDirections: Record<SortKey, 'ascending' | 'descending'> = {
  size: 'descending',
  name: 'ascending',
  modified: 'descending',
};

export function measurementStatus(finding: Finding): string {
  if (finding.issue) return finding.issue.replaceAll('_', ' ');
  if (finding.aliasOf) return 'Shared storage · counted once';
  return finding.complete ? 'Measured' : 'Partial measurement';
}
export function listedItems(finding: Finding): number {
  return finding.descendantItems - finding.foldedItems;
}
export function isSummarized(finding: Finding): boolean {
  return finding.kind === 'directory' && finding.detail === 'summarized';
}
export function foldedLabel(folded: FoldedRow, formatted: string): string {
  return `${folded.items.toLocaleString('en-US')} smaller ${folded.items === 1 ? 'item' : 'items'} · ${formatted} (not listed)`;
}
export function hasFoldedItems(folded: FoldedRow | null): folded is FoldedRow {
  return folded !== null && folded.items > 0;
}

function compareBytes(left: string, right: string): number {
  const leftValue = BigInt(left);
  const rightValue = BigInt(right);
  return leftValue < rightValue ? -1 : leftValue > rightValue ? 1 : 0;
}
/** Mirrors the native sort orders for flat investigation lists, which are resolved by id. */
export function sortFindings(findings: Finding[], sort: SortKey): Finding[] {
  return [...findings].sort((left, right) => {
    let comparison: number;
    switch (sort) {
      case 'size':
        comparison = compareBytes(right.allocatedBytes, left.allocatedBytes);
        break;
      case 'modified':
        comparison = (right.modifiedAtMs ?? -1) - (left.modifiedAtMs ?? -1);
        break;
      default:
        comparison = left.displayName
          .toLocaleLowerCase()
          .localeCompare(right.displayName.toLocaleLowerCase());
    }
    return comparison || left.id.localeCompare(right.id);
  });
}
