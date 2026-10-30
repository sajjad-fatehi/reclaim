import { afterEach, expect, test, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { fixtureFindings, fixtureProgress } from './fixtures';
import { nativeScanApi } from './contract';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

const wire = (value: unknown): unknown => JSON.parse(JSON.stringify(value));
const progress = (): unknown =>
  wire({
    ...fixtureProgress('complete'),
    retention: { nodes: 2, thresholdBytes: '1048576', budgetReached: false },
  });

afterEach(() => vi.mocked(invoke).mockReset());

test('summary and children decode folded fields and bounded shapes', async () => {
  const [root, file] = fixtureFindings();
  vi.mocked(invoke).mockResolvedValueOnce(
    wire({
      generation: 1,
      progress: progress(),
      roots: [root],
      topLevel: [file],
      topFiles: [
        {
          category: 'Other files',
          displayName: 'a.bin',
          displayPath: '/a.bin',
          logicalBytes: '5',
          allocatedBytes: '4096',
          modifiedAtMs: null,
          identity: '1:2',
          findingId: null,
        },
      ],
      issues: [{ issue: 'excluded', count: 1, examples: [file] }],
    }),
  );
  const summary = await nativeScanApi.summary(1);
  expect(summary.topFiles[0]?.category).toBe('Other files');
  expect(summary.roots[0]?.detail).toBe('full');
  expect(invoke).toHaveBeenCalledWith('scan_summary', { generation: 1 });

  vi.mocked(invoke).mockResolvedValueOnce(
    wire({
      generation: 1,
      parentId: root!.id,
      sort: 'size',
      total: 1,
      nextCursor: null,
      findings: [file],
      folded: { items: 3, logicalBytes: '3', allocatedBytes: '12288' },
    }),
  );
  const children = await nativeScanApi.children(1, root!.id, null, 'size');
  expect(children.folded?.items).toBe(3);
  expect(invoke).toHaveBeenLastCalledWith('scan_children', {
    generation: 1,
    parentId: root!.id,
    cursor: null,
    limit: 200,
    sort: 'size',
  });
});

test('malformed responses are rejected by the decoders', async () => {
  const [root] = fixtureFindings();
  vi.mocked(invoke).mockResolvedValueOnce({
    generation: 1,
    parentId: null,
    sort: 'path',
    total: 0,
    nextCursor: null,
    findings: [],
    folded: null,
  });
  await expect(nativeScanApi.children(1, null, null, 'size')).rejects.toThrow();
  vi.mocked(invoke).mockResolvedValueOnce([{ ...root, detail: 'partial' }]);
  await expect(nativeScanApi.findings(1, [root!.id])).rejects.toThrow();
  vi.mocked(invoke).mockResolvedValueOnce({
    ...(progress() as object),
    retention: null,
  });
  await expect(nativeScanApi.status(1)).rejects.toThrow();
});
