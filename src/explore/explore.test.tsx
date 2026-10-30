import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  within,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import type { Finding } from '../scan/contract';
import {
  createFixtureApi,
  fixtureFindings,
  fixtureLocation,
  fixtureProgress,
  serveScan,
} from '../scan/fixtures';
import { ScanSession } from '../scan/session';
import { ScanContext } from '../scan/useScanSession';
import { Explore } from './Explore';
import { ItemInspector } from './ItemInspector';
import { ancestors, browse, indexFindings } from './model';
import { parseInspection } from './contract';
import type { InspectionApi } from './contract';

function requiredFinding(entries: Finding[], index: number): Finding {
  const finding = entries[index];
  if (!finding) throw new Error('Missing synthetic fixture');
  return finding;
}
function fixturePair(): [Finding, Finding] {
  const entries = fixtureFindings();
  return [requiredFinding(entries, 0), requiredFinding(entries, 1)];
}
function findings(): Finding[] {
  const [root, file] = fixturePair();
  return [
    root,
    { ...root, id: '1:2', parentId: root.id, displayName: '開発' },
    { ...root, id: '1:3', parentId: root.id, displayName: 'other' },
    {
      ...file,
      id: '1:4',
      parentId: '1:2',
      displayName: '.hidden',
      allocatedBytes: '8192',
      complete: false,
      issue: 'permission_denied',
    },
    { ...file, id: '1:5', parentId: '1:2', displayName: 'same.txt' },
    { ...file, id: '1:6', parentId: '1:3', displayName: 'same.txt' },
    {
      ...file,
      id: '1:7',
      parentId: '1:3',
      displayName: '<script>不存在</script>',
      issue: 'vanished',
    },
  ];
}
const inspectionApi: InspectionApi = {
  inspect: async (_generation, findingId) => ({
    findingId,
    displayPath: '/SYNTHETIC/開発/same.txt',
    copyPath: '/SYNTHETIC/開発/same.txt',
    artifact: null,
    recognizedAtId: null,
  }),
  reveal: async () => undefined,
  copy: async () => undefined,
};
beforeEach(() => {
  vi.useFakeTimers();
  vi.stubGlobal(
    'matchMedia',
    vi.fn(() => ({
      matches: false,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    })),
  );
  Object.defineProperty(HTMLElement.prototype, 'animate', {
    configurable: true,
    value: vi.fn(() => ({ cancel: vi.fn() })),
  });
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

test('navigation, descendant search, stable ascending/descending ties, hidden and partial findings', () => {
  const index = indexFindings(findings());
  const options = {
    directoryId: '1:0',
    query: '',
    sort: 'name' as const,
    descending: false,
    findingIds: null,
  };
  expect(
    browse(index, options)
      .map((finding) => finding.id)
      .sort(),
  ).toEqual(['1:2', '1:3']);
  expect(ancestors(index, '1:2').map((finding) => finding.id)).toEqual([
    '1:0',
    '1:2',
  ]);
  const matches = browse(index, { ...options, query: 'same' });
  expect(matches.map((finding) => finding.id)).toEqual(['1:5', '1:6']);
  expect(index.paths.get('1:5')).not.toEqual(index.paths.get('1:6'));
  expect(
    browse(index, { ...options, directoryId: '1:2', query: 'same' }).map(
      (finding) => finding.id,
    ),
  ).toEqual(['1:5']);
  expect(browse(index, { ...options, query: 'missing' })).toEqual([]);
  expect(
    browse(index, { ...options, directoryId: '1:2', sort: 'size' }).map(
      (finding) => finding.id,
    ),
  ).toEqual(['1:5', '1:4']);
  expect(
    browse(index, {
      ...options,
      directoryId: '1:2',
      sort: 'size',
      descending: true,
    }).map((finding) => finding.id),
  ).toEqual(['1:4', '1:5']);
  expect(
    requiredFinding(browse(index, { ...options, query: '.hidden' }), 0).issue,
  ).toBe('permission_denied');
  expect(
    browse(index, { ...options, findingIds: new Set(['1:6']) }).map(
      (finding) => finding.id,
    ),
  ).toEqual(['1:6']);
});

test('100,000 findings stay bounded; long Unicode paths and exact byte sorting survive', () => {
  vi.useRealTimers();
  const [root, file] = fixturePair();
  const large = [
    root,
    ...Array.from({ length: 99_999 }, (_unused, index) => ({
      ...file,
      id: `1:${index + 1}`,
      displayName: `${'日本語'.repeat(100)}-${index}`,
      allocatedBytes: `${9007199254740993n + BigInt(index)}`,
    })),
  ];
  const started = performance.now();
  const index = indexFindings(large);
  const rows = browse(index, {
    directoryId: root.id,
    query: '日本語',
    sort: 'size',
    descending: true,
    findingIds: null,
  });
  expect(rows).toHaveLength(99_999);
  expect(requiredFinding(rows, 0).id).toBe('1:99999');
  expect(index.paths.get(requiredFinding(rows, 0).id)?.length).toBeGreaterThan(
    300,
  );
  console.info(
    `SYNTHETIC 100k index/search/sort: ${Math.round(performance.now() - started)}ms; page bounded at 50 rows`,
  );
});

async function setup(
  entries: Finding[] = findings(),
): Promise<{ session: ScanSession; disconnect: () => void }> {
  const api = createFixtureApi();
  api.current = async () => ({
    scope: { id: 'scope', roots: [fixtureLocation], exclusions: [] },
    progress: fixtureProgress(),
  });
  serveScan(api, fixtureProgress(), entries);
  const session = new ScanSession(api);
  const disconnect = session.connect();
  await vi.advanceTimersByTimeAsync(400);
  render(
    <ScanContext.Provider value={session}>
      <Explore api={inspectionApi} onOverview={vi.fn()} />
    </ScanContext.Provider>,
  );
  return { session, disconnect };
}

test('Open differs from Inspect; selection persists across filters/navigation and overview handoff resets', async () => {
  const { session, disconnect } = await setup();
  fireEvent.click(
    screen.getByRole('button', {
      name: `Open folder ${fixtureLocation.displayPath}`,
    }),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Open folder 開発' }));
  fireEvent.click(screen.getByRole('button', { name: 'Inspect same.txt' }));
  await act(async () => {
    await Promise.resolve();
  });
  expect(
    screen.getByRole('complementary', { name: 'Item details' }).textContent,
  ).toContain('Unknown owner');
  fireEvent.click(screen.getByRole('button', { name: 'Close Item details' }));
  expect(document.activeElement?.getAttribute('aria-label')).toBe(
    'Inspect same.txt',
  );
  fireEvent.change(screen.getByRole('searchbox'), {
    target: { value: '.hidden' },
  });
  expect(screen.queryByRole('button', { name: 'Inspect same.txt' })).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Up one folder' }));
  fireEvent.click(screen.getByRole('button', { name: 'Open folder 開発' }));
  expect(screen.getByRole('searchbox').getAttribute('value')).toBe('.hidden');
  fireEvent.click(screen.getByRole('button', { name: 'Reset filters' }));
  act(() =>
    session.investigate({
      generation: 1,
      category: 'Documents',
      findingIds: ['1:5'],
    }),
  );
  expect(screen.queryByRole('button', { name: 'Inspect .hidden' })).toBeNull();
  expect(screen.getByRole('button', { name: 'Inspect same.txt' })).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Reset filters' }));
  expect(session.getSnapshot().investigation).toBeNull();
  disconnect();
});

test('pagination renders 50 rows and resets empty filter without extra scan', async () => {
  const [root, file] = fixturePair();
  const { disconnect } = await setup([
    root,
    ...Array.from({ length: 123 }, (_unused, index) => ({
      ...file,
      id: `1:${index + 1}`,
      displayName: `file-${index}`,
    })),
  ]);
  fireEvent.click(
    screen.getByRole('button', { name: `Open folder ${root.displayName}` }),
  );
  const table = screen.getByRole('table');
  expect(within(table).getAllByRole('row')).toHaveLength(51);
  fireEvent.click(screen.getByRole('button', { name: 'Next page' }));
  expect(within(table).getAllByRole('row')).toHaveLength(51);
  fireEvent.change(screen.getByRole('searchbox'), {
    target: { value: 'no match' },
  });
  expect(screen.getByText(/No matching entries/)).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Reset filters' }));
  expect(within(table).getAllByRole('row')).toHaveLength(51);
  disconnect();
});

test('inspector copy/reveal are explicit identity actions; stale updates retain selection and safe errors', async () => {
  const finding = requiredFinding(findings(), 4);
  const api = {
    ...inspectionApi,
    reveal: vi.fn(async () => {
      throw { issue: 'vanished' };
    }),
    copy: vi.fn(async () => undefined),
  };
  const onClose = vi.fn();
  const result = render(
    <ItemInspector
      finding={finding}
      progress={fixtureProgress('partial')}
      stale={false}
      settled
      onClose={onClose}
      onInspectRelated={vi.fn()}
      api={api}
    />,
  );
  await act(async () => {
    await Promise.resolve();
  });
  expect(api.reveal).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Copy path' }));
  await act(async () => {
    await Promise.resolve();
  });
  expect(api.copy).toHaveBeenCalledWith('/SYNTHETIC/開発/same.txt');
  fireEvent.click(screen.getByRole('button', { name: 'Show in Finder' }));
  await act(async () => {
    await Promise.resolve();
  });
  expect(api.reveal).toHaveBeenCalledWith(1, finding.id);
  expect(screen.getByRole('status').textContent).toContain('no longer exists');
  result.rerender(
    <ItemInspector
      finding={{ ...finding, allocatedBytes: '12345' }}
      progress={fixtureProgress('cancelled')}
      stale
      settled
      onClose={onClose}
      onInspectRelated={vi.fn()}
      api={api}
    />,
  );
  expect(
    screen
      .getByRole('button', { name: 'Show in Finder' })
      .hasAttribute('disabled'),
  ).toBe(true);
  expect(screen.getByText(/12.0 KiB/)).toBeTruthy();
  expect(screen.getByText(/scan cancelled · stale/)).toBeTruthy();
  expect(() => parseInspection({ findingId: '1:1' })).toThrow();
});

test('filtered selection keeps details; closing disconnected invoker focuses scoped search', async () => {
  const { disconnect } = await setup();
  fireEvent.click(
    screen.getByRole('button', {
      name: `Open folder ${fixtureLocation.displayPath}`,
    }),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Open folder 開発' }));
  fireEvent.click(screen.getByRole('button', { name: 'Inspect same.txt' }));
  await act(async () => {
    await Promise.resolve();
  });
  fireEvent.change(screen.getByRole('searchbox'), {
    target: { value: '.hidden' },
  });
  expect(screen.getByRole('complementary').textContent).toContain('same.txt');
  fireEvent.click(screen.getByRole('button', { name: 'Close Item details' }));
  expect(document.activeElement).toBe(screen.getByRole('searchbox'));
  disconnect();
});

test('related evidence stays in same inspector; unknown byte paths disable exact copy; late results ignored', async () => {
  const first = requiredFinding(findings(), 4);
  const second = requiredFinding(findings(), 5);
  let resolveFirst:
    | ((details: Awaited<ReturnType<InspectionApi['inspect']>>) => void)
    | undefined;
  const delayed = new Promise<Awaited<ReturnType<InspectionApi['inspect']>>>(
    (resolve) => {
      resolveFirst = resolve;
    },
  );
  const related = vi.fn();
  const api: InspectionApi = {
    ...inspectionApi,
    inspect: async (_generation, findingId) =>
      findingId === first.id
        ? delayed
        : {
            findingId,
            displayPath: '"/SYNTHETIC/\\xFF"',
            copyPath: null,
            recognizedAtId: second.id,
            artifact: {
              findingId: second.id,
              family: 'javascript_project',
              nature: 'source',
              projectId: first.id,
              confidence: 'high',
              evidence: [{ findingId: first.id, method: 'manifest' }],
              consequence: 'Source data can be lost.',
              complete: false,
              regenerable: false,
            },
          },
  };
  const result = render(
    <ItemInspector
      finding={first}
      progress={fixtureProgress()}
      stale={false}
      settled
      onClose={vi.fn()}
      onInspectRelated={related}
      api={api}
    />,
  );
  result.rerender(
    <ItemInspector
      finding={second}
      progress={fixtureProgress()}
      stale={false}
      settled
      onClose={vi.fn()}
      onInspectRelated={related}
      api={api}
    />,
  );
  await act(async () => {
    await Promise.resolve();
  });
  expect(
    screen.getByRole('button', { name: 'Copy path' }).hasAttribute('disabled'),
  ).toBe(true);
  fireEvent.click(
    screen.getByRole('button', { name: 'Inspect related project' }),
  );
  expect(related).toHaveBeenCalledWith(first.id);
  await act(async () => {
    resolveFirst?.({
      findingId: first.id,
      displayPath: '/STALE/incorrect',
      copyPath: '/STALE/incorrect',
      artifact: null,
      recognizedAtId: null,
    });
    await Promise.resolve();
  });
  expect(screen.queryByText('/STALE/incorrect')).toBeNull();
  expect(screen.getByText('Source data can be lost.')).toBeTruthy();
});

test('Overview investigation starts at scan scope and reset restores prior folder', async () => {
  const { session, disconnect } = await setup();
  fireEvent.click(
    screen.getByRole('button', {
      name: `Open folder ${fixtureLocation.displayPath}`,
    }),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Open folder 開発' }));
  act(() =>
    session.investigate({
      generation: 1,
      category: 'Other files',
      findingIds: ['1:7'],
    }),
  );
  expect(
    screen.getByRole('button', { name: 'Inspect <script>不存在</script>' }),
  ).toBeTruthy();
  expect(screen.queryByRole('button', { name: 'Inspect .hidden' })).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Reset filters' }));
  expect(screen.getByRole('button', { name: 'Inspect .hidden' })).toBeTruthy();
  disconnect();
});

test('arrow keys move selection without stealing focus; Treemap layout exposes tiles', async () => {
  const { disconnect } = await setup();
  fireEvent.click(
    screen.getByRole('button', {
      name: `Open folder ${fixtureLocation.displayPath}`,
    }),
  );
  const first = screen.getAllByRole('button', { name: /^Inspect / })[0];
  if (!first) throw new Error('Missing row');
  first.focus();
  fireEvent.click(first);
  fireEvent.keyDown(first, { key: 'ArrowDown' });
  await act(async () => {
    await Promise.resolve();
  });
  const pressed = screen
    .getAllByRole('button', { name: /^Inspect / })
    .filter((button) => button.getAttribute('aria-pressed') === 'true');
  expect(pressed).toHaveLength(1);
  expect(pressed[0]).not.toBe(first);
  expect(document.activeElement).toBe(pressed[0]);
  fireEvent.click(screen.getByRole('button', { name: 'Treemap' }));
  expect(
    screen.getByRole('group', { name: 'Treemap of allocated size' }),
  ).toBeTruthy();
  disconnect();
});
