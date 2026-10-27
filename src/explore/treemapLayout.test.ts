import { expect, test } from 'vitest';
import { fixtureFindings } from '../scan/fixtures';
import { isDrawable, layoutTreemap } from './treemapLayout';

test('treemap areas are proportional, fill the canvas and skip unmeasured entries', () => {
  const [root, file] = fixtureFindings();
  if (!root || !file) throw new Error('Missing synthetic fixture');
  const findings = [
    { ...root, id: 'a', allocatedBytes: '600' },
    { ...root, id: 'b', allocatedBytes: '300' },
    { ...root, id: 'c', allocatedBytes: '100' },
    { ...file, id: 'denied', issue: 'permission_denied' },
    { ...file, id: 'empty', allocatedBytes: '0' },
  ];
  expect(findings.filter(isDrawable).map((finding) => finding.id)).toEqual([
    'a',
    'b',
    'c',
  ]);
  const tiles = layoutTreemap(findings);
  expect(tiles.map((tile) => tile.finding.id)).toEqual(['a', 'b', 'c']);
  const areas = tiles.map((tile) => (tile.width * tile.height) / 100);
  expect(areas.reduce((sum, area) => sum + area, 0)).toBeCloseTo(100, 5);
  expect(areas[0]).toBeCloseTo(60, 5);
  expect(areas[2]).toBeCloseTo(10, 5);
});
