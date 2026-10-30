import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  within,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import type { Finding, ScanApi } from '../scan/contract';
import {
  createFixtureApi,
  fixtureFindings,
  fixtureLocation,
  fixtureProgress,
} from '../scan/fixtures';
import { ScanSession } from '../scan/session';
import { ScanContext } from '../scan/useScanSession';
import { Explore } from './Explore';
import { ItemInspector } from './ItemInspector';
import { foldedLabel, sortFindings } from './model';
import { readThrottled, staleRead } from './reads';
import { createTreeApi } from './treeApi';
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

function mount(
  entries: Finding[],
  folded: Record<string, { items: number; allocatedBytes: string }> = {},
): { api: ScanApi; session: ScanSession; disconnect: () => void } {
  const api = createTreeApi(createFixtureApi(), entries, folded);
  api.current = async () => ({
    scope: { id: 'scope', roots: [fixtureLocation], exclusions: [] },
    progress: fixtureProgress(),
  });
  api.page = async (_generation, offset) => ({
    progress: fixtureProgress(),
    offset,
    nextOffset: null,
    findings: [],
  });
  const session = new ScanSession(api);
  const disconnect = session.connect();
  return { api, session, disconnect };
}
async function settle(milliseconds = 400): Promise<void> {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(milliseconds);
  });
}
async function setup(
  entries: Finding[] = findings(),
  folded: Record<string, { items: number; allocatedBytes: string }> = {},
): Promise<{
  api: ScanApi;
  session: ScanSession;
  disconnect: () => void;
}> {
  const mounted = mount(entries, folded);
  await vi.advanceTimersByTimeAsync(400);
  render(
    <ScanContext.Provider value={mounted.session}>
      <Explore api={inspectionApi} onOverview={vi.fn()} />
    </ScanContext.Provider>,
  );
  await settle();
  return mounted;
}
async function openRoot(): Promise<void> {
  fireEvent.click(
    screen.getByRole('button', {
      name: `Open folder ${fixtureLocation.displayPath}`,
    }),
  );
  await settle();
}

test('flat lists sort like the native orders and folded copy states the remainder', () => {
  const [root, file] = fixturePair();
  const list = [
    {
      ...file,
      id: 'b',
      displayName: 'Bravo',
      allocatedBytes: '9007199254740993',
    },
    {
      ...file,
      id: 'a',
      displayName: 'alpha',
      allocatedBytes: '9007199254740992',
    },
    {
      ...root,
      id: 'c',
      displayName: 'charlie',
      allocatedBytes: '5',
      modifiedAtMs: 7,
    },
  ];
  expect(sortFindings(list, 'size').map((finding) => finding.id)).toEqual([
    'b',
    'a',
    'c',
  ]);
  expect(sortFindings(list, 'name').map((finding) => finding.id)).toEqual([
    'a',
    'b',
    'c',
  ]);
  expect(sortFindings(list, 'modified')[0]?.id).toBe('c');
  expect(
    foldedLabel(
      { items: 1234, logicalBytes: '1', allocatedBytes: '1' },
      '1.0 GiB',
    ),
  ).toBe('1,234 smaller items · 1.0 GiB (not listed)');
});

test('readThrottled retries rate_limited, serializes reads and drops stale scopes', async () => {
  const owner = {};
  const order: string[] = [];
  const limited = vi
    .fn<() => Promise<string>>()
    .mockRejectedValueOnce({ code: 'rate_limited' })
    .mockRejectedValueOnce({ code: 'rate_limited' })
    .mockResolvedValue('ok');
  const first = readThrottled(owner, async () => {
    order.push('first');
    return limited();
  });
  const second = readThrottled(owner, async () => {
    order.push('second');
    return 'later';
  });
  await vi.advanceTimersByTimeAsync(400);
  expect(await first).toBe('ok');
  expect(await second).toBe('later');
  expect(limited).toHaveBeenCalledTimes(3);
  expect(order).toEqual(['first', 'first', 'first', 'second']);
  const skipped = vi.fn(async () => 'never');
  await expect(readThrottled(owner, skipped, () => false)).rejects.toBe(
    staleRead,
  );
  expect(skipped).not.toHaveBeenCalled();
  await expect(
    readThrottled(owner, async () => {
      throw { code: 'busy' };
    }),
  ).rejects.toEqual({ code: 'busy' });
});

test('rate_limited listings are retried transparently and never shown as an error', async () => {
  const mounted = mount(findings());
  const children = vi.spyOn(mounted.api, 'children');
  children.mockRejectedValueOnce({ code: 'rate_limited' });
  children.mockRejectedValueOnce({ code: 'rate_limited' });
  await vi.advanceTimersByTimeAsync(400);
  render(
    <ScanContext.Provider value={mounted.session}>
      <Explore api={inspectionApi} onOverview={vi.fn()} />
    </ScanContext.Provider>,
  );
  expect(screen.queryByText(/rate/i)).toBeNull();
  await settle(500);
  expect(children).toHaveBeenCalledTimes(3);
  expect(
    screen.getByRole('button', {
      name: `Open folder ${fixtureLocation.displayPath}`,
    }),
  ).toBeTruthy();
  expect(screen.queryByText(/rate.limit|Listing unavailable/i)).toBeNull();
  mounted.disconnect();
});

test('other listing failures offer a retry', async () => {
  const mounted = mount(findings());
  const children = vi.spyOn(mounted.api, 'children');
  children.mockRejectedValueOnce({ code: 'internal' });
  await vi.advanceTimersByTimeAsync(400);
  render(
    <ScanContext.Provider value={mounted.session}>
      <Explore api={inspectionApi} onOverview={vi.fn()} />
    </ScanContext.Provider>,
  );
  await settle();
  expect(screen.getByText(/Listing unavailable/)).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Retry' }));
  await settle();
  expect(screen.queryByText(/Listing unavailable/)).toBeNull();
  expect(
    screen.getByRole('button', {
      name: `Open folder ${fixtureLocation.displayPath}`,
    }),
  ).toBeTruthy();
  mounted.disconnect();
});

test('server-driven sort and navigation; breadcrumbs and up-one-folder', async () => {
  const { api, disconnect } = await setup();
  const children = vi.spyOn(api, 'children');
  await openRoot();
  expect(children).toHaveBeenLastCalledWith(1, '1:0', null, 'size');
  fireEvent.click(screen.getByRole('button', { name: 'Name' }));
  await settle();
  expect(children).toHaveBeenLastCalledWith(1, '1:0', null, 'name');
  expect(
    screen
      .getByRole('columnheader', { name: /Name/ })
      .getAttribute('aria-sort'),
  ).toBe('ascending');
  fireEvent.click(screen.getByRole('button', { name: 'Open folder 開発' }));
  await settle();
  expect(
    screen.getByRole('navigation', { name: 'Folder breadcrumbs' }).textContent,
  ).toContain('開発');
  expect(screen.getByRole('button', { name: 'Inspect .hidden' })).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Up one folder' }));
  await settle();
  expect(screen.getByRole('button', { name: 'Inspect other' })).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Scan locations' }));
  await settle();
  expect(
    screen.getByRole('button', {
      name: `Open folder ${fixtureLocation.displayPath}`,
    }),
  ).toBeTruthy();
  disconnect();
});

test('summarized directory ends with a folded row and treemap tile that cannot be selected', async () => {
  const { disconnect } = await setup(findings(), {
    '1:2': { items: 4321, allocatedBytes: '3221225472' },
  });
  await openRoot();
  expect(document.querySelector('.folded-row')).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Open folder 開発' }));
  await settle();
  const folded = document.querySelector('.folded-row');
  expect(folded?.textContent).toContain(
    '4,321 smaller items · 3.0 GiB (not listed)',
  );
  expect(folded?.querySelector('button')).toBeNull();
  expect(screen.getAllByRole('button', { name: /^Inspect / }).length).toBe(2);
  fireEvent.click(screen.getByRole('button', { name: 'Treemap' }));
  const tile = screen.getByRole('img', {
    name: '4,321 smaller items · 3.0 GiB (not listed)',
  });
  expect(tile.tagName).not.toBe('BUTTON');
  fireEvent.click(tile);
  expect(screen.queryByRole('complementary')).toBeNull();
  disconnect();
});

test('inspector explains summarized directories and shared storage', () => {
  const [root, file] = fixturePair();
  render(
    <ItemInspector
      finding={{
        ...root,
        detail: 'summarized',
        descendantItems: 1000,
        foldedItems: 900,
      }}
      progress={fixtureProgress()}
      stale={false}
      settled
      onClose={vi.fn()}
      onInspectRelated={vi.fn()}
      api={inspectionApi}
    />,
  );
  expect(
    screen.getByText('Contains 1,000 items; 100 listed; remainder summarized.'),
  ).toBeTruthy();
  cleanup();
  render(
    <ItemInspector
      finding={{ ...file, aliasOf: '16777221:42' }}
      progress={fixtureProgress()}
      stale={false}
      settled
      onClose={vi.fn()}
      onInspectRelated={vi.fn()}
      api={inspectionApi}
    />,
  );
  expect(
    screen.getByText(/shares storage with an item counted elsewhere/),
  ).toBeTruthy();
  expect(screen.queryByText(/16777221:42/)).toBeNull();
});

test('search is native, debounced, scope-disclosed and never claims completeness', async () => {
  const { api, disconnect } = await setup();
  const search = vi.spyOn(api, 'search');
  expect(screen.getByText('Search listed items')).toBeTruthy();
  const box = screen.getByRole('searchbox');
  fireEvent.change(box, { target: { value: 's' } });
  fireEvent.change(box, { target: { value: 'sa' } });
  fireEvent.change(box, { target: { value: 'same' } });
  await settle(100);
  expect(search).not.toHaveBeenCalled();
  await settle(500);
  expect(search).toHaveBeenCalledTimes(1);
  expect(search).toHaveBeenCalledWith(1, 'same');
  expect(
    screen.getAllByRole('button', { name: 'Inspect same.txt' }),
  ).toHaveLength(2);
  expect(document.body.textContent).toContain('2 listed matches');
  expect(document.body.textContent).toContain('never a complete count');
  expect(document.body.textContent).not.toMatch(
    /all matches|complete results/i,
  );
  fireEvent.change(box, { target: { value: 'nothing-here' } });
  await settle(600);
  expect(document.body.textContent).toContain(
    'No listed item matches. Smaller items summarized into folders are not searched.',
  );
  fireEvent.click(screen.getByRole('button', { name: 'Reset filters' }));
  await settle();
  expect(
    screen.getByRole('button', {
      name: `Open folder ${fixtureLocation.displayPath}`,
    }),
  ).toBeTruthy();
  disconnect();
});

test('Open differs from Inspect; selection persists across navigation and overview handoff resets', async () => {
  const { session, disconnect } = await setup();
  await openRoot();
  fireEvent.click(screen.getByRole('button', { name: 'Open folder 開発' }));
  await settle();
  fireEvent.click(screen.getByRole('button', { name: 'Inspect same.txt' }));
  await settle();
  expect(
    screen.getByRole('complementary', { name: 'Item details' }).textContent,
  ).toContain('Unknown owner');
  fireEvent.click(screen.getByRole('button', { name: 'Close Item details' }));
  expect(document.activeElement?.getAttribute('aria-label')).toBe(
    'Inspect same.txt',
  );
  act(() =>
    session.investigate({
      generation: 1,
      category: 'Documents',
      findingIds: ['1:5'],
    }),
  );
  await settle();
  expect(screen.queryByRole('button', { name: 'Inspect .hidden' })).toBeNull();
  expect(screen.getByRole('button', { name: 'Inspect same.txt' })).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Reset filters' }));
  expect(session.getSnapshot().investigation).toBeNull();
  await settle();
  expect(screen.getByRole('button', { name: 'Inspect .hidden' })).toBeTruthy();
  disconnect();
});

test('pagination renders 50 rows and loads further chunks on demand', async () => {
  const [root, file] = fixturePair();
  const many = Array.from({ length: 260 }, (_unused, index) => ({
    ...file,
    id: `1:${index + 1}`,
    parentId: root.id,
    displayName: `file-${index}`,
    allocatedBytes: String(1000 - index),
  }));
  const { api, disconnect } = await setup([root, ...many]);
  const children = vi.spyOn(api, 'children');
  await openRoot();
  const table = screen.getByRole('table');
  expect(within(table).getAllByRole('row')).toHaveLength(51);
  for (let step = 0; step < 4; step += 1) {
    fireEvent.click(screen.getByRole('button', { name: 'Next page' }));
    await settle();
  }
  expect(children).toHaveBeenCalledTimes(2);
  expect(children).toHaveBeenLastCalledWith(1, '1:0', '200', 'size');
  expect(within(table).getAllByRole('row')).toHaveLength(51);
  fireEvent.click(screen.getByRole('button', { name: 'Next page' }));
  await settle();
  expect(within(table).getAllByRole('row')).toHaveLength(11);
  expect(
    screen.getByRole('button', { name: 'Next page' }).hasAttribute('disabled'),
  ).toBe(true);
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

test('searched selection keeps details; closing a disconnected invoker focuses search', async () => {
  const { disconnect } = await setup();
  await openRoot();
  fireEvent.click(screen.getByRole('button', { name: 'Open folder 開発' }));
  await settle();
  fireEvent.click(screen.getByRole('button', { name: 'Inspect same.txt' }));
  await settle();
  fireEvent.change(screen.getByRole('searchbox'), {
    target: { value: '.hidden' },
  });
  await settle(600);
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

test('Overview investigation resolves ids natively and reset restores prior folder', async () => {
  const { api, session, disconnect } = await setup();
  const resolve = vi.spyOn(api, 'findings');
  await openRoot();
  fireEvent.click(screen.getByRole('button', { name: 'Open folder 開発' }));
  await settle();
  act(() =>
    session.investigate({
      generation: 1,
      category: 'Other files',
      findingIds: ['1:7'],
    }),
  );
  await settle();
  expect(resolve).toHaveBeenCalledWith(1, ['1:7']);
  expect(
    screen.getByRole('button', { name: 'Inspect <script>不存在</script>' }),
  ).toBeTruthy();
  expect(screen.queryByRole('button', { name: 'Inspect .hidden' })).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Reset filters' }));
  await settle();
  expect(screen.getByRole('button', { name: 'Inspect .hidden' })).toBeTruthy();
  disconnect();
});

test('arrow keys move selection without stealing focus; Treemap layout exposes tiles', async () => {
  const { disconnect } = await setup();
  await openRoot();
  const first = screen.getAllByRole('button', { name: /^Inspect / })[0];
  if (!first) throw new Error('Missing row');
  first.focus();
  fireEvent.click(first);
  fireEvent.keyDown(first, { key: 'ArrowDown' });
  await settle();
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
