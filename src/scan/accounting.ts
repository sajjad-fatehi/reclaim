import type { Finding } from './contract';

export const categoryNames = [
  'Documents',
  'Media',
  'Archives',
  'Other files',
] as const;
export type CategoryName = (typeof categoryNames)[number];
export interface Category {
  name: CategoryName;
  logicalBytes: bigint;
  allocatedBytes: bigint;
  findingIds: string[];
}
export function categoryFor(finding: Finding): CategoryName {
  const extension = finding.displayName.split('.').pop()?.toLowerCase() ?? '';
  if (
    [
      'pdf',
      'txt',
      'md',
      'doc',
      'docx',
      'xls',
      'xlsx',
      'csv',
      'ppt',
      'pptx',
      'rtf',
    ].includes(extension)
  )
    return 'Documents';
  if (
    [
      'png',
      'jpg',
      'jpeg',
      'webp',
      'gif',
      'heic',
      'mov',
      'mp4',
      'mp3',
      'wav',
      'm4a',
      'mkv',
    ].includes(extension)
  )
    return 'Media';
  if (['zip', 'gz', 'tar', 'xz', '7z', 'dmg', 'iso'].includes(extension))
    return 'Archives';
  return 'Other files';
}
export function categorize(findings: Finding[]): Category[] {
  const categories: Category[] = categoryNames.map((name) => ({
    name,
    logicalBytes: 0n,
    allocatedBytes: 0n,
    findingIds: [],
  }));
  for (const finding of findings) {
    if (
      finding.kind !== 'file' ||
      finding.aliasOf !== null ||
      finding.issue === 'excluded'
    )
      continue;
    const category = categories.find(
      (category) => category.name === categoryFor(finding),
    );
    if (!category) continue;
    category.logicalBytes += BigInt(finding.logicalBytes);
    category.allocatedBytes += BigInt(finding.allocatedBytes);
    category.findingIds.push(finding.id);
  }
  return categories.sort((left, right) =>
    left.allocatedBytes === right.allocatedBytes
      ? left.name.localeCompare(right.name)
      : left.allocatedBytes > right.allocatedBytes
        ? -1
        : 1,
  );
}
export function formatBytes(value: string | bigint | null): string {
  if (value === null) return 'Unavailable';
  const bytes = BigInt(value);
  const units = ['B', 'KiB', 'MiB', 'GiB', 'TiB', 'PiB', 'EiB'];
  let divisor = 1n;
  let index = 0;
  while (bytes >= divisor * 1024n && index < units.length - 1) {
    divisor *= 1024n;
    index += 1;
  }
  const tenths = (bytes * 10n) / divisor;
  return `${tenths / 10n}${index > 0 ? `.${tenths % 10n}` : ''} ${units[index]}`;
}
export function segmentWidth(bytes: string, total: string): string {
  const denominator = BigInt(total);
  return denominator > 0n
    ? `${Number((BigInt(bytes) * 10000n) / denominator) / 100}%`
    : '0%';
}
