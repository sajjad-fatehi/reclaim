import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  within,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { ScanSession } from './session';
import {
  createFixtureApi,
  fixtureCategories,
  fixtureFindings,
  fixtureLocation,
  fixtureProgress,
  serveScan,
  summaryOf,
} from './fixtures';
import { ScanProvider } from './context';
import { Overview } from './Overview';
import { categorize, formatBytes } from './accounting';
import type { CategoryTotal, ScanApi } from './contract';

beforeEach(() => vi.useFakeTimers());
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
});
async function initialize(
  api: ScanApi,
): Promise<{ session: ScanSession; disconnect: () => void }> {
  const session = new ScanSession(api);
  const disconnect = session.connect();
  await vi.advanceTimersByTimeAsync(0);
  return { session, disconnect };
}

test('scope/exclusion/start/progressive/terminal/cancel are user actions; restart never scans', async () => {
  const api = createFixtureApi();
  const start = vi.spyOn(api, 'start');
  const register = vi.spyOn(api, 'register');
  const cancel = vi.spyOn(api, 'cancel');
  let cancelled = false;
  vi.spyOn(api, 'status').mockImplementation(async (generation) =>
    fixtureProgress(cancelled ? 'cancelled' : 'running', generation),
  );
  vi.spyOn(api, 'summary').mockImplementation(async (generation) =>
    summaryOf(
      fixtureProgress('cancelled', generation),
      fixtureFindings(generation),
    ),
  );
  cancel.mockImplementation(async (generation) => {
    cancelled = true;
    return fixtureProgress('running', generation);
  });
  const { session, disconnect } = await initialize(api);
  expect(start).not.toHaveBeenCalled();
  session.editPreferences({
    exclusions: [{ relativePath: 'cache', subtree: true }],
  });
  await session.start();
  expect(register).toHaveBeenCalledWith(
    [fixtureLocation.id],
    [{ relativePath: 'cache', subtree: true }],
  );
  await vi.advanceTimersByTimeAsync(160);
  expect(session.getSnapshot().snapshot?.progress.status).toBe('running');
  expect(session.getSnapshot().snapshot?.settled).toBe(false);
  session.editPreferences({ exclusions: [] });
  expect(session.getSnapshot().snapshot?.scope.exclusions).toHaveLength(1);
  await session.cancel();
  await vi.advanceTimersByTimeAsync(320);
  expect(session.getSnapshot().snapshot?.progress.status).toBe('cancelled');
  expect(session.getSnapshot().snapshot?.settled).toBe(true);
  disconnect();
  const restarted = await initialize(api);
  expect(start).toHaveBeenCalledTimes(1);
  expect(restarted.session.getSnapshot().preferences.roots).toHaveLength(1);
  expect(restarted.session.getSnapshot().snapshot).toBeNull();
  restarted.disconnect();
});

test('old generation responses never overwrite current observations; errors retain data', async () => {
  const api = createFixtureApi();
  const { session, disconnect } = await initialize(api);
  await session.start();
  await vi.advanceTimersByTimeAsync(320);
  expect(session.getSnapshot().snapshot?.settled).toBe(true);
  vi.spyOn(api, 'status').mockResolvedValue(fixtureProgress('complete', 1));
  await session.start();
  await vi.advanceTimersByTimeAsync(160);
  expect(session.getSnapshot().snapshot?.progress.generation).toBe(2);
  expect(session.getSnapshot().previous?.findings).toHaveLength(2);
  expect(session.getSnapshot().stale).toBe(true);
  expect(session.getSnapshot().error).toMatch(/older scan generation/);
  disconnect();
});

test('rate limits retry transparently and settling takes a handful of IPC calls', async () => {
  const api = createFixtureApi();
  const status = vi.spyOn(api, 'status');
  const summary = vi.spyOn(api, 'summary');
  const children = vi.spyOn(api, 'children');
  const search = vi.spyOn(api, 'search');
  const findings = vi.spyOn(api, 'findings');
  status.mockRejectedValueOnce({ code: 'rate_limited' });
  status.mockResolvedValueOnce(fixtureProgress('running'));
  summary.mockRejectedValueOnce({ code: 'rate_limited' });
  const { session, disconnect } = await initialize(api);
  await session.start();
  await vi.advanceTimersByTimeAsync(1_000);
  const state = session.getSnapshot();
  expect(state.error).toBeNull();
  expect(state.snapshot?.settled).toBe(true);
  expect(state.snapshot?.summary?.generation).toBe(1);
  const calls = status.mock.calls.length + summary.mock.calls.length;
  expect(calls).toBeLessThanOrEqual(6);
  expect(summary).toHaveBeenCalledTimes(2);
  expect(children).not.toHaveBeenCalled();
  expect(search).not.toHaveBeenCalled();
  expect(findings).not.toHaveBeenCalled();
  await vi.advanceTimersByTimeAsync(5_000);
  expect(status.mock.calls.length + summary.mock.calls.length).toBe(calls);
  disconnect();
});

test('a summary that is always rate limited ends in a visible error, not endless scanning', async () => {
  const api = createFixtureApi();
  vi.spyOn(api, 'summary').mockRejectedValue({ code: 'rate_limited' });
  const { session, disconnect } = await initialize(api);
  await session.start();
  await vi.advanceTimersByTimeAsync(30_000);
  const state = session.getSnapshot();
  expect(state.snapshot?.settled).toBe(false);
  expect(state.error).toMatch(/connection interrupted/);
  disconnect();
});

test('a malformed or oversized summary ends in a visible error with retry', async () => {
  const api = createFixtureApi();
  vi.spyOn(api, 'summary').mockRejectedValueOnce({ code: 'invalid_response' });
  const { session, disconnect } = await initialize(api);
  await session.start();
  await vi.advanceTimersByTimeAsync(1_000);
  expect(session.getSnapshot().error).toMatch(/connection interrupted/);
  session.resume();
  await vi.advanceTimersByTimeAsync(1_000);
  expect(session.getSnapshot().snapshot?.settled).toBe(true);
  expect(session.getSnapshot().error).toBeNull();
  disconnect();
});

test('real Overview fixture flow edits scope and exclusions, starts explicitly and hands off category', async () => {
  const api = createFixtureApi();
  const start = vi.spyOn(api, 'start');
  const investigate = vi.fn();
  render(
    <ScanProvider api={api}>
      <Overview onInvestigate={investigate} />
    </ScanProvider>,
  );
  await act(async () => vi.advanceTimersByTimeAsync(0));
  expect(start).not.toHaveBeenCalled();
  expect(screen.queryByLabelText('Scan locations and exclusions')).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Edit' }));
  fireEvent.click(screen.getByRole('button', { name: 'Add folder…' }));
  await act(async () => vi.advanceTimersByTimeAsync(0));
  fireEvent.click(
    screen.getByRole('button', { name: 'Remove /SYNTHETIC/Added' }),
  );
  fireEvent.change(screen.getByLabelText('Relative path'), {
    target: { value: 'cache' },
  });
  fireEvent.click(screen.getByRole('button', { name: 'Add exclusion' }));
  expect(screen.getByText('cache · Subtree')).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Start scan' }));
  await act(async () => vi.advanceTimersByTimeAsync(320));
  expect(start).toHaveBeenCalledTimes(1);
  expect(screen.getByRole('heading', { name: 'Scan complete' })).toBeTruthy();
  const volume = screen.getByRole('region', { name: 'Volume fixture usage' });
  expect(within(volume).getByText('581.9 KiB')).toBeTruthy();
  expect(volume.querySelectorAll('.segment-category')).toHaveLength(4);
  expect(
    within(volume).getByText('Measured categories: Documents 4.0 KiB.'),
  ).toBeTruthy();
  fireEvent.click(
    screen.getByRole('button', { name: 'Investigate Documents' }),
  );
  expect(investigate).toHaveBeenCalledTimes(1);
});

test.each(['empty', 'denied', 'cancelled', 'failed'] as const)(
  'renders distinct %s coverage',
  async (outcome) => {
    const api = createFixtureApi();
    const root = fixtureFindings()[0]!;
    const findings =
      outcome === 'empty'
        ? [{ ...root, logicalBytes: '0', allocatedBytes: '0' }]
        : [
            {
              ...root,
              complete: false,
              issue: 'permission_denied',
              logicalBytes: '0',
              allocatedBytes: '0',
            },
          ];
    const status =
      outcome === 'empty'
        ? 'complete'
        : outcome === 'denied'
          ? 'partial'
          : outcome;
    const progress = {
      ...fixtureProgress(status),
      logicalBytes: '0',
      allocatedBytes: '0',
      issueCount: outcome === 'empty' ? 0 : 1,
    };
    progress.volumes = [
      {
        ...progress.volumes[0]!,
        scopeLogicalBytes: '0',
        scopeAllocatedBytes: '0',
        categories: fixtureCategories('0').map((category) => ({
          ...category,
          logicalBytes: '0',
          count: 0,
        })),
      },
    ];
    serveScan(api, progress, findings, []);
    render(
      <ScanProvider api={api}>
        <Overview onInvestigate={() => undefined} />
      </ScanProvider>,
    );
    await act(async () => vi.advanceTimersByTimeAsync(0));
    fireEvent.click(screen.getByRole('button', { name: 'Start scan' }));
    await act(async () => vi.advanceTimersByTimeAsync(320));
    if (outcome === 'empty')
      expect(screen.getByText(/No measured files in this scope/)).toBeTruthy();
    if (outcome === 'denied')
      expect(screen.getByText(/No readable results/)).toBeTruthy();
    if (outcome === 'cancelled')
      expect(
        screen.getByText(/Cancellation preserves measured entries/),
      ).toBeTruthy();
    if (outcome === 'failed')
      expect(screen.getByText(/Native scan failed/)).toBeTruthy();
  },
);

test('corrupt settings and missing roots stay actionable without auto-start; failed persistence retains edits', async () => {
  const api = createFixtureApi();
  vi.spyOn(api, 'loadPreferences').mockResolvedValue({
    appearance: 'system',
    configured: true,
    corrupt: true,
    exclusions: [],
    roots: [
      {
        key: 'missing',
        displayPath: '/SYNTHETIC/Disconnected',
        location: null,
      },
    ],
  });
  vi.spyOn(api, 'savePreferences').mockRejectedValue(new Error('private path'));
  const start = vi.spyOn(api, 'start');
  render(
    <ScanProvider api={api}>
      <Overview onInvestigate={() => undefined} />
    </ScanProvider>,
  );
  await act(async () => vi.advanceTimersByTimeAsync(0));
  fireEvent.click(screen.getByRole('button', { name: 'Edit' }));
  expect(screen.getByText(/Saved settings are corrupt/)).toBeTruthy();
  expect(screen.getByText(/Unavailable · volume missing/)).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Start scan' }));
  expect(start).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Save settings' }));
  await act(async () => vi.advanceTimersByTimeAsync(0));
  expect(screen.getByText(/Settings could not be saved/)).toBeTruthy();
  expect(screen.queryByText(/private path/)).toBeNull();
});

test('native category totals sum to the volume scope, preserve u64 precision and unknown metrics', () => {
  const progress = fixtureProgress();
  const volume = progress.volumes[0]!;
  const categories = categorize([volume]);
  expect(categories.map((category) => category.name)).toEqual([
    'Documents',
    'Archives',
    'Media',
    'Other files',
  ]);
  expect(
    categories.reduce((total, category) => total + category.allocatedBytes, 0n),
  ).toBe(BigInt(volume.scopeAllocatedBytes));
  const large: CategoryTotal[] = [
    {
      name: 'Other files',
      logicalBytes: '9007199254740993',
      allocatedBytes: '9007199254740993',
      count: 1,
    },
    ...fixtureCategories(),
  ];
  const [first] = categorize([{ ...volume, categories: large }]);
  expect(first?.allocatedBytes).toBe(9007199254740993n);
  expect(formatBytes(null)).toBe('Unavailable');
  expect(formatBytes('0')).toBe('0 B');
});

test('running scans show live counters and category totals without a final pass', async () => {
  const api = createFixtureApi();
  const progress = {
    ...fixtureProgress('running'),
    entries: 12_345,
  };
  vi.spyOn(api, 'status').mockResolvedValue(progress);
  const summary = vi.spyOn(api, 'summary');
  render(
    <ScanProvider api={api}>
      <Overview onInvestigate={() => undefined} />
    </ScanProvider>,
  );
  await act(async () => vi.advanceTimersByTimeAsync(0));
  fireEvent.click(screen.getByRole('button', { name: 'Start scan' }));
  await act(async () => vi.advanceTimersByTimeAsync(640));
  expect(screen.getByText(/12,345 entries observed/)).toBeTruthy();
  expect(screen.getAllByText(/Live category totals/).length).toBeGreaterThan(0);
  expect(
    screen.getByRole('button', { name: 'Investigate Documents' }),
  ).toHaveProperty('disabled', true);
  expect(summary).not.toHaveBeenCalled();
});

test('retention is plain information and untracked sharing is a qualified unknown', async () => {
  const api = createFixtureApi();
  const progress = { ...fixtureProgress(), sharingUntracked: true };
  serveScan(api, progress, fixtureFindings());
  render(
    <ScanProvider api={api}>
      <Overview onInvestigate={() => undefined} />
    </ScanProvider>,
  );
  await act(async () => vi.advanceTimersByTimeAsync(0));
  fireEvent.click(screen.getByRole('button', { name: 'Start scan' }));
  await act(async () => vi.advanceTimersByTimeAsync(320));
  expect(screen.getByText(/true figure is unknown/)).toBeTruthy();
  expect(screen.getByText(/summarized in their folders/)).toBeTruthy();
  expect(document.body.textContent).not.toMatch(
    /\bcap(ped)?\b.*entr|entry limit/i,
  );
});

test('appearance edits preserve scan freshness and migrate legacy preference until first native save', async () => {
  localStorage.setItem('reclaim.appearance', 'dark');
  const api = createFixtureApi();
  const { session, disconnect } = await initialize(api);
  expect(session.getSnapshot().preferences.appearance).toBe('dark');
  await session.start();
  await vi.advanceTimersByTimeAsync(320);
  session.editPreferences({ appearance: 'light' });
  expect(session.getSnapshot().stale).toBe(false);
  expect(await session.save()).toBe(true);
  expect(session.getSnapshot().snapshot?.progress.generation).toBe(1);
  disconnect();
  localStorage.clear();
});

test('unavailable volume metrics stay unknown in rendered Overview', async () => {
  const api = createFixtureApi();
  const progress = fixtureProgress();
  progress.volumes = [
    {
      volume: {
        ...fixtureLocation.volume,
        totalBytes: null,
        usedBytes: null,
        freeBytes: null,
        availableBytes: null,
        metricIssue: 'permission_denied',
      },
      scopeLogicalBytes: '5',
      scopeAllocatedBytes: '4096',
      categories: fixtureCategories(),
      unscannedOrUnexplainedBytes: null,
    },
  ];
  serveScan(api, progress, fixtureFindings());
  render(
    <ScanProvider api={api}>
      <Overview onInvestigate={() => undefined} />
    </ScanProvider>,
  );
  await act(async () => vi.advanceTimersByTimeAsync(0));
  fireEvent.click(screen.getByRole('button', { name: 'Start scan' }));
  await act(async () => vi.advanceTimersByTimeAsync(320));
  const volume = screen.getByRole('region', { name: 'Volume fixture usage' });
  expect(within(volume).getAllByText('Unavailable').length).toBe(3);
  expect(within(volume).getByText(/missing or inconsistent/)).toBeTruthy();
  expect(volume.querySelector('.usage-strip')).toBeNull();
});

test('capped native contributions propagate independently through category sums without precision loss', () => {
  const maximum = '18446744073709551615';
  const volume = fixtureProgress().volumes[0]!;
  const categories = categorize([
    {
      ...volume,
      categories: [
        {
          name: 'Documents',
          logicalBytes: maximum,
          allocatedBytes: '12',
          count: 2,
        },
        { name: 'Media', logicalBytes: '3', allocatedBytes: maximum, count: 1 },
        { name: 'Documents', logicalBytes: '1', allocatedBytes: '0', count: 0 },
      ],
    },
  ]);
  const documents = categories.find(
    (category) => category.name === 'Documents',
  )!;
  const media = categories.find((category) => category.name === 'Media')!;
  expect(documents.logicalBytes).toBe(18446744073709551616n);
  expect(documents.logicalCapped).toBe(true);
  expect(documents.allocatedCapped).toBe(false);
  expect(formatBytes(documents.logicalBytes, documents.logicalCapped)).toBe(
    'At least 16.0 EiB (capped)',
  );
  expect(formatBytes(documents.allocatedBytes, documents.allocatedCapped)).toBe(
    '12 B',
  );
  expect(media.logicalCapped).toBe(false);
  expect(media.allocatedCapped).toBe(true);
  expect(formatBytes(maximum)).toBe('At least 15.9 EiB (capped)');
  expect(formatBytes('18446744073709551614')).toBe('15.9 EiB');
});

test.each(['allocation', 'volume'] as const)(
  'capped %s renders lower bounds and suppresses numerically balanced volume segments',
  async (source) => {
    const maximum = '18446744073709551615';
    const api = createFixtureApi();
    const progress = fixtureProgress();
    const allocated = source === 'allocation' ? maximum : '5';
    progress.logicalBytes = maximum;
    progress.allocatedBytes = allocated;
    progress.volumes = [
      {
        volume: {
          ...fixtureLocation.volume,
          totalBytes: maximum,
          usedBytes: maximum,
          freeBytes: '0',
          availableBytes: '0',
        },
        scopeLogicalBytes: maximum,
        scopeAllocatedBytes: allocated,
        categories: [
          {
            name: 'Documents',
            logicalBytes: maximum,
            allocatedBytes: allocated,
            count: 1,
          },
        ],
        unscannedOrUnexplainedBytes:
          source === 'allocation' ? '0' : '18446744073709551610',
      },
    ];
    serveScan(api, progress, fixtureFindings());
    render(
      <ScanProvider api={api}>
        <Overview onInvestigate={() => undefined} />
      </ScanProvider>,
    );
    await act(async () => vi.advanceTimersByTimeAsync(0));
    fireEvent.click(screen.getByRole('button', { name: 'Start scan' }));
    await act(async () => vi.advanceTimersByTimeAsync(320));
    const volume = screen.getByRole('region', { name: 'Volume fixture usage' });
    expect(volume.querySelector('.usage-strip')).toBeNull();
    expect(
      within(volume).getByText(/capped accounting cannot reconcile/),
    ).toBeTruthy();
    expect(
      within(volume).getByText('At least 15.9 EiB (capped) total'),
    ).toBeTruthy();
    expect(
      within(volume).getByText('Unavailable (capped accounting)'),
    ).toBeTruthy();
    const documents = screen
      .getByRole('button', { name: 'Investigate Documents' })
      .closest('tr');
    if (!documents) throw new Error('Missing Documents category row');
    expect(
      within(documents).getAllByText('At least 15.9 EiB (capped)'),
    ).toHaveLength(source === 'allocation' ? 2 : 1);
    if (source === 'volume')
      expect(within(documents).getByText('5 B')).toBeTruthy();
  },
);

test('hero scan button label follows state, Cancel works, Edit reveals settings', async () => {
  const api = createFixtureApi();
  const cancel = vi.spyOn(api, 'cancel');
  render(
    <ScanProvider api={api}>
      <Overview onInvestigate={() => undefined} />
    </ScanProvider>,
  );
  await act(async () => vi.advanceTimersByTimeAsync(0));
  expect(screen.getByText(/\d+ locations? · \d+ exclusions?/)).toBeTruthy();
  const edit = screen.getByRole('button', { name: 'Edit' });
  expect(edit.getAttribute('aria-expanded')).toBe('false');
  fireEvent.click(edit);
  expect(edit.getAttribute('aria-expanded')).toBe('true');
  expect(screen.getByLabelText('Scan locations and exclusions')).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Start scan' }));
  await act(async () => vi.advanceTimersByTimeAsync(10));
  expect(screen.queryByRole('button', { name: 'Start scan' })).toBeNull();
  expect(screen.getByText('Measuring metadata').textContent).toMatch(
    /Measuring/,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Cancel scan' }));
  await act(async () => vi.advanceTimersByTimeAsync(640));
  expect(cancel).toHaveBeenCalled();
  expect(screen.getByRole('button', { name: 'Scan again' })).toBeTruthy();
});
