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
  fixtureFindings,
  fixtureLocation,
  fixtureProgress,
} from './fixtures';
import { ScanProvider } from './context';
import { Overview } from './Overview';
import { categorize, formatBytes } from './accounting';
import type { ScanApi } from './contract';

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
  vi.spyOn(api, 'page').mockImplementation(async (generation, offset) => ({
    progress: fixtureProgress(cancelled ? 'cancelled' : 'running', generation),
    offset,
    nextOffset: null,
    findings: fixtureFindings(generation),
  }));
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
  expect(session.getSnapshot().snapshot?.findings).toHaveLength(2);
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
  const page = vi.spyOn(api, 'page');
  const { session, disconnect } = await initialize(api);
  await session.start();
  await vi.advanceTimersByTimeAsync(320);
  expect(session.getSnapshot().snapshot?.settled).toBe(true);
  page.mockResolvedValue({
    progress: fixtureProgress('complete', 1),
    offset: 0,
    nextOffset: null,
    findings: fixtureFindings(1),
  });
  await session.start();
  await vi.advanceTimersByTimeAsync(160);
  expect(session.getSnapshot().snapshot?.progress.generation).toBe(2);
  expect(session.getSnapshot().previous?.findings).toHaveLength(2);
  expect(session.getSnapshot().stale).toBe(true);
  expect(session.getSnapshot().error).toMatch(/older scan generation/);
  disconnect();
});

test('terminal pages re-read updated ancestors and rate limit retries remain serialized', async () => {
  const api = createFixtureApi();
  const page = vi.spyOn(api, 'page');
  page.mockRejectedValueOnce({ code: 'rate_limited' });
  page.mockResolvedValueOnce({
    progress: fixtureProgress('running'),
    offset: 0,
    nextOffset: null,
    findings: [
      {
        ...fixtureFindings()[0]!,
        logicalBytes: '0',
        allocatedBytes: '0',
        complete: false,
      },
    ],
  });
  const { session, disconnect } = await initialize(api);
  await session.start();
  await vi.advanceTimersByTimeAsync(640);
  expect(page).toHaveBeenCalledTimes(4);
  expect(session.getSnapshot().snapshot?.findings[0]?.logicalBytes).toBe('5');
  expect(session.getSnapshot().snapshot?.settled).toBe(true);
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
    vi.spyOn(api, 'page').mockImplementation(async (_generation, offset) => ({
      progress: {
        ...fixtureProgress(status),
        logicalBytes: '0',
        allocatedBytes: '0',
        issueCount: outcome === 'empty' ? 0 : 1,
      },
      offset,
      nextOffset: null,
      findings,
    }));
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

test('categories partition unique files, preserve u64 precision and unknown metrics', () => {
  const file = fixtureFindings()[1]!;
  const categories = categorize([
    ...fixtureFindings(),
    { ...file, id: '1:2', aliasOf: file.id },
    {
      ...file,
      id: '1:3',
      displayName: 'large.bin',
      logicalBytes: '9007199254740993',
      allocatedBytes: '9007199254740993',
    },
  ]);
  expect(
    categories.reduce((total, category) => total + category.logicalBytes, 0n),
  ).toBe(9007199254740998n);
  expect(
    categories.reduce((total, category) => total + category.allocatedBytes, 0n),
  ).toBe(9007199254745089n);
  expect(formatBytes(null)).toBe('Unavailable');
  expect(formatBytes('0')).toBe('0 B');
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
      unscannedOrUnexplainedBytes: null,
    },
  ];
  vi.spyOn(api, 'page').mockImplementation(async (_generation, offset) => ({
    progress,
    offset,
    nextOffset: null,
    findings: fixtureFindings(),
  }));
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
  const file = fixtureFindings()[1]!;
  const findings = [
    { ...file, logicalBytes: maximum, allocatedBytes: '5' },
    { ...file, id: '1:2', logicalBytes: '1', allocatedBytes: '7' },
    {
      ...file,
      id: '1:3',
      displayName: 'synthetic.jpg',
      logicalBytes: '3',
      allocatedBytes: maximum,
    },
    {
      ...file,
      id: '1:4',
      aliasOf: file.id,
      logicalBytes: maximum,
      allocatedBytes: maximum,
    },
  ];
  const categories = categorize(findings);
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
    const file = {
      ...fixtureFindings()[1]!,
      allocatedBytes: source === 'allocation' ? maximum : '5',
      logicalBytes: maximum,
    };
    const findings = [fixtureFindings()[0]!, file];
    progress.logicalBytes = maximum;
    progress.allocatedBytes = file.allocatedBytes;
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
        scopeAllocatedBytes: file.allocatedBytes,
        unscannedOrUnexplainedBytes:
          source === 'allocation' ? '0' : '18446744073709551610',
      },
    ];
    vi.spyOn(api, 'page').mockImplementation(async (_generation, offset) => ({
      progress,
      offset,
      nextOffset: null,
      findings,
    }));
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
  expect(screen.getByText('Scanning…').textContent).toMatch(/Scanning/);
  fireEvent.click(screen.getByRole('button', { name: 'Cancel scan' }));
  await act(async () => vi.advanceTimersByTimeAsync(640));
  expect(cancel).toHaveBeenCalled();
  expect(screen.getByRole('button', { name: 'Scan again' })).toBeTruthy();
});
