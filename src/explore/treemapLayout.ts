import type { Finding } from '../scan/contract';

export interface TreemapTile {
  finding: Finding;
  left: number;
  top: number;
  width: number;
  height: number;
}
interface Weighted {
  finding: Finding;
  area: number;
}
interface Rectangle {
  left: number;
  top: number;
  width: number;
  height: number;
}

const layoutWidth = 200;
const layoutHeight = 100;

export function isDrawable(finding: Finding): boolean {
  return !finding.issue && !finding.aliasOf && weightOf(finding) > 0;
}
function weightOf(finding: Finding): number {
  try {
    return Number(BigInt(finding.allocatedBytes));
  } catch {
    return 0;
  }
}
function worstRatio(row: Weighted[], side: number): number {
  const sum = row.reduce((total, item) => total + item.area, 0);
  const areas = row.map((item) => item.area);
  const largest = Math.max(...areas);
  const smallest = Math.min(...areas);
  return Math.max(
    (side * side * largest) / (sum * sum),
    (sum * sum) / (side * side * smallest),
  );
}
function placeRow(
  row: Weighted[],
  rectangle: Rectangle,
  tiles: TreemapTile[],
): Rectangle {
  const sum = row.reduce((total, item) => total + item.area, 0);
  const vertical = rectangle.width >= rectangle.height;
  const thickness = vertical ? sum / rectangle.height : sum / rectangle.width;
  let offset = 0;
  for (const item of row) {
    const length = item.area / thickness;
    tiles.push({
      finding: item.finding,
      left: vertical ? rectangle.left : rectangle.left + offset,
      top: vertical ? rectangle.top + offset : rectangle.top,
      width: vertical ? thickness : length,
      height: vertical ? length : thickness,
    });
    offset += length;
  }
  return vertical
    ? {
        left: rectangle.left + thickness,
        top: rectangle.top,
        width: rectangle.width - thickness,
        height: rectangle.height,
      }
    : {
        left: rectangle.left,
        top: rectangle.top + thickness,
        width: rectangle.width,
        height: rectangle.height - thickness,
      };
}

/**
 * Squarified treemap. Tile area is proportional to allocated size within the
 * drawn set. Output is in percent of a 2:1 canvas. Only drawable findings are
 * laid out; callers must list the rest as not drawn.
 */
export function layoutTreemap(findings: Finding[]): TreemapTile[] {
  const drawable = findings
    .filter(isDrawable)
    .sort((left, right) => weightOf(right) - weightOf(left));
  const total = drawable.reduce((sum, finding) => sum + weightOf(finding), 0);
  if (total === 0) return [];
  const queue: Weighted[] = drawable.map((finding) => ({
    finding,
    area: (weightOf(finding) / total) * layoutWidth * layoutHeight,
  }));
  const tiles: TreemapTile[] = [];
  let rectangle: Rectangle = {
    left: 0,
    top: 0,
    width: layoutWidth,
    height: layoutHeight,
  };
  let row: Weighted[] = [];
  for (const item of queue) {
    const side = Math.min(rectangle.width, rectangle.height);
    if (
      row.length === 0 ||
      worstRatio([...row, item], side) <= worstRatio(row, side)
    ) {
      row.push(item);
      continue;
    }
    rectangle = placeRow(row, rectangle, tiles);
    row = [item];
  }
  if (row.length > 0) placeRow(row, rectangle, tiles);
  return tiles.map((tile) => ({
    finding: tile.finding,
    left: (tile.left / layoutWidth) * 100,
    top: (tile.top / layoutHeight) * 100,
    width: (tile.width / layoutWidth) * 100,
    height: (tile.height / layoutHeight) * 100,
  }));
}
