import type { Category } from './accounting';

export interface SunburstItem {
  id: string;
  label: string;
  bytes: bigint;
}
export interface SunburstCategory {
  name: Category['name'];
  index: number;
  bytes: bigint;
  items: SunburstItem[];
}

const itemsPerCategory = 3;

function compareBytesDescending(left: bigint, right: bigint): number {
  if (left === right) return 0;
  return left > right ? -1 : 1;
}

/**
 * Splits each category into its largest native top files plus one remainder
 * item so the outer ring always sums to the inner ring.
 */
export function buildSunburstCategories(
  categories: Category[],
  categoryIndex: (name: Category['name']) => number,
): SunburstCategory[] {
  return categories
    .filter((category) => category.allocatedBytes > 0n)
    .map((category) => {
      const files = category.files
        .map((file) => ({
          id: file.identity,
          label: file.displayName,
          bytes: BigInt(file.allocatedBytes),
        }))
        .filter((item) => item.bytes > 0n)
        .sort((left, right) => compareBytesDescending(left.bytes, right.bytes));
      const largest = files.slice(0, itemsPerCategory);
      const largestBytes = largest.reduce(
        (total, item) => total + item.bytes,
        0n,
      );
      const remainder = category.allocatedBytes - largestBytes;
      const items =
        remainder > 0n
          ? [
              ...largest,
              {
                id: `${category.name}:remainder`,
                label: `Other ${category.name.toLowerCase()}`,
                bytes: remainder,
              },
            ]
          : largest;
      return {
        name: category.name,
        index: categoryIndex(category.name),
        bytes: category.allocatedBytes,
        items,
      };
    });
}

export interface ArcGeometry {
  startAngle: number;
  endAngle: number;
}

const fullTurn = Math.PI * 2;
const gapAngle = 0.012;

export function layoutArcs(
  weights: bigint[],
  total: bigint,
  startAngle = 0,
  span = fullTurn,
): ArcGeometry[] {
  if (total <= 0n) return [];
  const gap = weights.length > 1 ? gapAngle : 0;
  let cursor = startAngle;
  return weights.map((weight) => {
    const sweep = (Number((weight * 100000n) / total) / 100000) * span;
    const arc = {
      startAngle: cursor + gap / 2,
      endAngle: Math.max(cursor + gap / 2, cursor + sweep - gap / 2),
    };
    cursor += sweep;
    return arc;
  });
}

function point(
  radius: number,
  angle: number,
  center: number,
): [number, number] {
  return [center + radius * Math.sin(angle), center - radius * Math.cos(angle)];
}

export function describeArc(
  innerRadius: number,
  outerRadius: number,
  { startAngle, endAngle }: ArcGeometry,
  center: number,
): string {
  const sweep = endAngle - startAngle;
  if (sweep >= fullTurn - 0.001) {
    const [outerTopX, outerTopY] = point(outerRadius, 0, center);
    const [outerBottomX, outerBottomY] = point(outerRadius, Math.PI, center);
    const [innerTopX, innerTopY] = point(innerRadius, 0, center);
    const [innerBottomX, innerBottomY] = point(innerRadius, Math.PI, center);
    return [
      `M${outerTopX} ${outerTopY}`,
      `A${outerRadius} ${outerRadius} 0 1 1 ${outerBottomX} ${outerBottomY}`,
      `A${outerRadius} ${outerRadius} 0 1 1 ${outerTopX} ${outerTopY}Z`,
      `M${innerTopX} ${innerTopY}`,
      `A${innerRadius} ${innerRadius} 0 1 0 ${innerBottomX} ${innerBottomY}`,
      `A${innerRadius} ${innerRadius} 0 1 0 ${innerTopX} ${innerTopY}Z`,
    ].join('');
  }
  const largeArc = sweep > Math.PI ? 1 : 0;
  const [outerStartX, outerStartY] = point(outerRadius, startAngle, center);
  const [outerEndX, outerEndY] = point(outerRadius, endAngle, center);
  const [innerEndX, innerEndY] = point(innerRadius, endAngle, center);
  const [innerStartX, innerStartY] = point(innerRadius, startAngle, center);
  return [
    `M${outerStartX} ${outerStartY}`,
    `A${outerRadius} ${outerRadius} 0 ${largeArc} 1 ${outerEndX} ${outerEndY}`,
    `L${innerEndX} ${innerEndY}`,
    `A${innerRadius} ${innerRadius} 0 ${largeArc} 0 ${innerStartX} ${innerStartY}Z`,
  ].join('');
}
