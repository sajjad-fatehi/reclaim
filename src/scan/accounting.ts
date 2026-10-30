import type { CategoryName, TopFile, VolumeAccounting } from './contract';

export const maximumNativeBytes = 18446744073709551615n;

export function isCappedBytes(value: string | bigint | null): boolean {
  return value !== null && BigInt(value) === maximumNativeBytes;
}

export const categoryNames: CategoryName[] = [
  'Documents',
  'Media',
  'Archives',
  'Other files',
];
export interface Category {
  name: CategoryName;
  logicalBytes: bigint;
  allocatedBytes: bigint;
  logicalCapped: boolean;
  allocatedCapped: boolean;
  count: number;
  files: TopFile[];
}
/**
 * Sums native per-volume category totals. Native code owns classification and
 * the invariant that categories add up to the volume's allocated scope; every
 * category is always present so the table keeps stable rows.
 */
export function categorize(
  volumes: VolumeAccounting[],
  topFiles: TopFile[] = [],
): Category[] {
  const categories: Category[] = categoryNames.map((name) => ({
    name,
    logicalBytes: 0n,
    allocatedBytes: 0n,
    logicalCapped: false,
    allocatedCapped: false,
    count: 0,
    files: topFiles.filter((file) => file.category === name),
  }));
  for (const total of volumes.flatMap((volume) => volume.categories)) {
    const category = categories.find(
      (candidate) => candidate.name === total.name,
    );
    if (!category) continue;
    category.logicalCapped ||= isCappedBytes(total.logicalBytes);
    category.allocatedCapped ||= isCappedBytes(total.allocatedBytes);
    category.logicalBytes += BigInt(total.logicalBytes);
    category.allocatedBytes += BigInt(total.allocatedBytes);
    category.count += total.count;
  }
  return categories.sort((left, right) =>
    left.allocatedBytes === right.allocatedBytes
      ? left.name.localeCompare(right.name)
      : left.allocatedBytes > right.allocatedBytes
        ? -1
        : 1,
  );
}
export function formatBytes(
  value: string | bigint | null,
  capped = false,
): string {
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
  const formatted = `${tenths / 10n}${index > 0 ? `.${tenths % 10n}` : ''} ${units[index]}`;
  return capped || isCappedBytes(bytes)
    ? `At least ${formatted} (capped)`
    : formatted;
}
export function segmentWidth(bytes: string, total: string): string {
  const denominator = BigInt(total);
  return denominator > 0n
    ? `${Number((BigInt(bytes) * 10000n) / denominator) / 100}%`
    : '0%';
}
