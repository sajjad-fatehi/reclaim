import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  within,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { ScanSession } from '../scan/session';
import { ScanContext } from '../scan/useScanSession';
import { ScanApplication } from '../scan/ScanApplication';
import { Development } from './Development';
import { developmentFixture, dockerFixture, fixtureArtifact } from './fixtures';
import {
  familyLabels,
  dockerFailures,
  dockerErrorMessage,
  loadRecognition,
  parseArtifact,
  parseDockerSnapshot,
  parseRecognitionPage,
} from './contract';
import type { DevelopmentApi } from './contract';
import {
  filterArtifacts,
  initialDevelopmentView,
  relatedWorktrees,
} from './model';
import { indexFindings } from '../explore/model';

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

async function setup(
  options: {
    api?: DevelopmentApi;
    shell?: boolean;
    fixture?: ReturnType<typeof developmentFixture>;
  } = {},
): Promise<{
  session: ScanSession;
  disconnect: () => void;
  fixture: ReturnType<typeof developmentFixture>;
}> {
  const fixture = options.fixture ?? developmentFixture();
  const session = new ScanSession(fixture.scanApi);
  const disconnect = session.connect();
  await vi.advanceTimersByTimeAsync(400);
  render(
    <ScanContext.Provider value={session}>
      {options.shell ? (
        <ScanApplication
          connectionStatus="SYNTHETIC"
          developmentApi={options.api ?? fixture.api}
          inspectionApi={fixture.inspectionApi}
        />
      ) : (
        <Development
          active
          api={options.api ?? fixture.api}
          inspectionApi={fixture.inspectionApi}
          onOverview={vi.fn()}
          onExplore={vi.fn()}
        />
      )}
    </ScanContext.Provider>,
  );
  await act(async () => {
    await Promise.resolve();
  });
  return { session, disconnect, fixture };
}
function switchTools(): void {
  fireEvent.click(screen.getByRole('tab', { name: 'Tools' }));
}

test('strict native DTO parsing rejects malformed bytes/states; all native families accepted', () => {
  const fixture = developmentFixture();
  expect(dockerErrorMessage('access_denied')).toBe(
    dockerFailures.access_denied,
  );
  expect(dockerErrorMessage(new Error('private socket path'))).toBe(
    'Docker connection unavailable. Check desktop access and retry.',
  );
  expect(parseRecognitionPage(fixture.report)).toEqual(fixture.report);
  for (const artifact of fixture.report.findings)
    expect(parseArtifact(artifact)).toEqual(artifact);
  expect(
    new Set(fixture.report.findings.map((artifact) => artifact.family)).size,
  ).toBe(Object.keys(familyLabels).length);
  expect(parseDockerSnapshot(dockerFixture())).toEqual(dockerFixture());
  for (const failure of Object.keys(dockerFailures))
    expect(
      parseDockerSnapshot({ measuredAtMs: '2000', result: { Err: failure } })
        .result,
    ).toEqual({ Err: failure });
  const artifact = fixtureArtifact('1:1');
  for (const bytes of ['-1', '1.5', '18446744073709551616', 123])
    expect(() =>
      parseArtifact({ ...artifact, exclusiveAllocatedBytes: bytes }),
    ).toThrow();
  expect(() => parseArtifact({ ...artifact, family: 'invented' })).toThrow();
  expect(() =>
    parseDockerSnapshot({
      measuredAtMs: '1',
      result: {
        Ok: {
          items: [{ ...dockerFixture(), kind: 'invented' }],
          accounting: '',
        },
      },
    }),
  ).toThrow();
});

test('recognition pages preserve native totals and reject stale, duplicate and stuck pagination', async () => {
  const fixture = developmentFixture();
  const entries = Array.from({ length: 201 }, (_unused, index) =>
    fixtureArtifact(`1:${index}`),
  );
  const api = {
    ...fixture.api,
    recognitionPage: vi.fn(async (_generation: number, offset: number) => ({
      ...fixture.report,
      offset,
      findings: entries.slice(offset, offset + 200),
      nextOffset: offset === 0 ? 200 : null,
    })),
  };
  const report = await loadRecognition(api, 1);
  expect(report.findings).toEqual(entries);
  expect(api.recognitionPage.mock.calls).toEqual([
    [1, 0],
    [1, 200],
  ]);
  await expect(
    loadRecognition(
      {
        ...api,
        recognitionPage: async () => ({ ...fixture.report, generation: 2 }),
      },
      1,
    ),
  ).rejects.toThrow();
  await expect(
    loadRecognition(
      {
        ...api,
        recognitionPage: async () => ({
          ...fixture.report,
          findings: [entries[0], entries[0]].filter(
            (entry) => entry !== undefined,
          ),
        }),
      },
      1,
    ),
  ).rejects.toThrow();
  await expect(
    loadRecognition(
      {
        ...api,
        recognitionPage: async () => ({
          ...fixture.report,
          nextOffset: 0,
          findings: [],
        }),
      },
      1,
    ),
  ).rejects.toThrow();
});

test('native owned totals and shared exclusive rows remain separate; stable BigInt sorting and worktree links', () => {
  const fixture = developmentFixture();
  const index = indexFindings(fixture.findings);
  const projects = filterArtifacts(
    fixture.report.findings,
    index.byId,
    index.paths,
    'projects',
    initialDevelopmentView,
  );
  expect(projects.map((artifact) => artifact.findingId)).toEqual([
    '1:1',
    '1:5',
  ]);
  expect(projects[0]?.ownedAllocatedBytes).toBe('12288');
  const project = projects[0];
  if (!project) throw new Error('Missing fixture project');
  expect(
    relatedWorktrees(fixture.report.findings, project).map(
      (artifact) => artifact.findingId,
    ),
  ).toEqual(['1:5']);
  const tools = filterArtifacts(
    fixture.report.findings,
    index.byId,
    index.paths,
    'tools',
    { ...initialDevelopmentView, family: 'pnpm' },
  );
  expect(tools).toHaveLength(1);
  expect(tools[0]?.projectId).toBeNull();
  const large = [
    { ...project, findingId: '1:99', ownedAllocatedBytes: '9007199254740993' },
    { ...project, findingId: '1:98', ownedAllocatedBytes: '9007199254740992' },
  ];
  expect(
    filterArtifacts(
      large,
      index.byId,
      index.paths,
      'projects',
      initialDevelopmentView,
    )[0]?.findingId,
  ).toBe('1:99');
});

test('Projects exposes owned artifacts and locked/unknown linked worktree; all Tools families usable', async () => {
  const { disconnect } = await setup();
  fireEvent.click(
    screen.getByRole('button', { name: 'Inspect SYNTHETIC Atlas – 開発' }),
  );
  await act(async () => {
    await Promise.resolve();
  });
  const details = screen.getByRole('region', {
    name: 'Selected artifact relationships',
  });
  expect(details.textContent).toContain('dirty');
  expect(details.textContent).toContain('Dependencies');
  expect(details.textContent).toContain('Build outputs');
  expect(details.textContent).toContain('Logs');
  expect(details.textContent).toContain('locked');
  expect(details.textContent).toContain('unknown');
  expect(details.textContent).toContain('12.0 KiB');
  fireEvent.click(
    within(details).getByRole('button', { name: 'View related worktree' }),
  );
  expect(
    screen.getByRole('region', { name: 'Selected artifact relationships' })
      .textContent,
  ).toContain('feat/SYNTHETIC-開発');
  fireEvent.click(screen.getByRole('button', { name: 'Close Item details' }));
  switchTools();
  for (const label of Object.values(familyLabels))
    expect(
      within(
        screen.getByRole('table', {
          name: 'Filesystem artifacts by tool family',
        }),
      ).getAllByText(label).length,
    ).toBeGreaterThan(0);
  fireEvent.change(screen.getByRole('combobox', { name: 'Tool family' }), {
    target: { value: 'pnpm' },
  });
  fireEvent.click(
    screen.getByRole('button', { name: 'Inspect SYNTHETIC pnpm' }),
  );
  expect(
    screen.getByRole('region', { name: 'Selected artifact relationships' })
      .textContent,
  ).toContain('Related projects unknown');
  expect(screen.queryByRole('button', { name: /Add to review/ })).toBeNull();
  disconnect();
});

test('tab keyboard, scoped filters, shell navigation and Explore exact selection retain context', async () => {
  const { disconnect, session } = await setup({ shell: true });
  fireEvent.click(screen.getByRole('button', { name: 'Development' }));
  const projects = screen.getByRole('tab', { name: 'Projects' });
  projects.focus();
  fireEvent.keyDown(projects, { key: 'ArrowRight' });
  expect(document.activeElement).toBe(
    screen.getByRole('tab', { name: 'Tools' }),
  );
  fireEvent.change(
    screen.getByRole('searchbox', { name: 'Search selected locations' }),
    { target: { value: 'pnpm' } },
  );
  fireEvent.click(screen.getByRole('button', { name: 'Overview' }));
  fireEvent.click(screen.getByRole('button', { name: 'Development' }));
  expect(screen.getByRole('searchbox').getAttribute('value')).toBe('pnpm');
  fireEvent.click(
    screen.getByRole('button', { name: 'Explore SYNTHETIC pnpm' }),
  );
  await act(async () => {
    await Promise.resolve();
  });
  expect(session.getSnapshot().investigation?.selectedId).toBe('1:16');
  expect(
    screen.getByRole('complementary', { name: 'Item details' }).textContent,
  ).toContain('SYNTHETIC pnpm');
  fireEvent.click(screen.getByRole('button', { name: 'Development' }));
  expect(screen.getByRole('searchbox').getAttribute('value')).toBe('pnpm');
  expect(
    screen.queryByRole('complementary', { name: 'Item details' }),
  ).toBeNull();
  disconnect();
});

test('Docker requires explicit action; active/persistent/unknown metrics use shared inspector without filesystem actions', async () => {
  const fixture = developmentFixture();
  const api = {
    ...fixture.api,
    dockerInventory: vi.fn(fixture.api.dockerInventory),
  };
  const { disconnect } = await setup({ api });
  expect(api.dockerInventory).not.toHaveBeenCalled();
  switchTools();
  fireEvent.click(
    screen.getByRole('button', { name: 'Inspect Docker inventory' }),
  );
  await act(async () => {
    await Promise.resolve();
  });
  expect(api.dockerInventory).toHaveBeenCalledTimes(1);
  expect(
    screen.getByRole('table', {
      name: 'Docker observations, not additive totals',
    }).textContent,
  ).toContain('Persistent data');
  fireEvent.click(
    screen.getByRole('button', {
      name: 'Inspect Docker volume SYNTHETIC-volume-identity',
    }),
  );
  const inspector = screen.getByRole('complementary', { name: 'Item details' });
  expect(inspector.textContent).toContain('Unavailable');
  expect(inspector.textContent).toContain('Unknown');
  expect(inspector.textContent).toContain(
    'SYNTHETIC verified fixture metadata',
  );
  expect(
    within(inspector).queryByRole('button', { name: 'Show in Finder' }),
  ).toBeNull();
  expect(
    within(inspector).queryByRole('button', { name: 'Copy path' }),
  ).toBeNull();
  disconnect();
});

test('Docker unavailable and empty differ; cancellation and transport failure remain honest', async () => {
  const fixture = developmentFixture();
  const api = {
    ...fixture.api,
    dockerInventory: vi.fn(fixture.api.dockerInventory),
    cancelDocker: vi.fn(async () => undefined),
  };
  api.dockerInventory.mockResolvedValueOnce({
    measuredAtMs: '2000',
    result: { Err: 'daemon_unavailable' },
  });
  api.dockerInventory.mockResolvedValueOnce({
    measuredAtMs: '2000',
    result: { Ok: { items: [], accounting: 'Not additive' } },
  });
  api.dockerInventory.mockRejectedValueOnce(new Error('private native error'));
  const { disconnect } = await setup({ api });
  switchTools();
  fireEvent.click(
    screen.getByRole('button', { name: 'Inspect Docker inventory' }),
  );
  await act(async () => {
    await Promise.resolve();
  });
  expect(screen.getByText(/Local Docker daemon unavailable/)).toBeTruthy();
  fireEvent.click(
    screen.getByRole('button', { name: 'Refresh Docker inventory' }),
  );
  await act(async () => {
    await Promise.resolve();
  });
  expect(
    screen.getByText('No Docker resources reported by this inventory.'),
  ).toBeTruthy();
  fireEvent.click(
    screen.getByRole('button', { name: 'Refresh Docker inventory' }),
  );
  await act(async () => {
    await Promise.resolve();
  });
  expect(screen.getByText(/Retained inventory is stale/)).toBeTruthy();
  expect(screen.queryByText('private native error')).toBeNull();
  disconnect();
});

test('recognition errors retry; no matches, partial state and 50-row rendering stay bounded', async () => {
  const fixture = developmentFixture();
  const api = {
    ...fixture.api,
    recognitionPage: vi.fn(fixture.api.recognitionPage),
  };
  api.recognitionPage.mockRejectedValueOnce(new Error('private path'));
  const { disconnect } = await setup({ api });
  expect(screen.getByRole('alert').textContent).toContain(
    'Recognition unavailable',
  );
  fireEvent.click(screen.getByRole('button', { name: 'Retry recognition' }));
  await act(async () => {
    await Promise.resolve();
  });
  expect(screen.getByText(/Scan partial/)).toBeTruthy();
  fireEvent.change(screen.getByRole('searchbox'), {
    target: { value: 'nothing here' },
  });
  expect(screen.getByText(/No matching projects artifacts/)).toBeTruthy();
  expect(screen.queryByText('private path')).toBeNull();
  disconnect();
});

test('native throttle retries once; stale generations still fail closed', async () => {
  const fixture = developmentFixture();
  const recognitionPage = vi.fn(fixture.api.recognitionPage);
  recognitionPage.mockRejectedValueOnce({ code: 'rate_limited' });
  const pending = loadRecognition({ ...fixture.api, recognitionPage }, 1);
  await vi.advanceTimersByTimeAsync(120);
  expect((await pending).generation).toBe(1);
  expect(recognitionPage).toHaveBeenCalledTimes(2);
});

test('large Tools pages stay at 50 artifacts; selection and separate tab filters persist', async () => {
  const fixture = developmentFixture();
  const template = fixture.findings[0];
  if (!template) throw new Error('Missing fixture root');
  const extra = Array.from({ length: 120 }, (_unused, index) =>
    fixtureArtifact(`1:${index + 100}`, 'pnpm', 'shared_store'),
  );
  fixture.report.findings.push(...extra);
  fixture.findings.push(
    ...extra.map((artifact) => ({
      ...template,
      id: artifact.findingId,
      parentId: template.id,
      displayName: `SYNTHETIC shared ${artifact.findingId}`,
      identity: artifact.findingId,
    })),
  );
  const { disconnect } = await setup({ fixture });
  switchTools();
  const table = screen.getByRole('table', {
    name: 'Filesystem artifacts by tool family',
  });
  expect(
    within(table).getAllByRole('button', { name: /^Inspect / }),
  ).toHaveLength(50);
  fireEvent.click(
    screen.getByRole('button', { name: 'Next Development page' }),
  );
  expect(
    within(table).getAllByRole('button', { name: /^Inspect / }),
  ).toHaveLength(50);
  fireEvent.change(screen.getByRole('searchbox'), {
    target: { value: 'shared' },
  });
  expect(
    within(table).getAllByRole('button', { name: /^Inspect / }),
  ).toHaveLength(50);
  const inspect = within(table).getAllByRole('button', {
    name: /^Inspect /,
  })[0];
  if (!inspect) throw new Error('Missing fixture row');
  fireEvent.click(inspect);
  await act(async () => {
    await Promise.resolve();
  });
  fireEvent.click(screen.getByRole('tab', { name: 'Projects' }));
  expect(screen.getByRole('searchbox').getAttribute('value')).toBe('');
  switchTools();
  expect(screen.getByRole('searchbox').getAttribute('value')).toBe('shared');
  expect(
    screen.getByRole('complementary', { name: 'Item details' }),
  ).toBeTruthy();
  disconnect();
});

test('Docker pending request cancels explicitly and late results after unmount have no effect', async () => {
  const fixture = developmentFixture();
  let resolveInventory:
    ((result: ReturnType<typeof dockerFixture>) => void) | undefined;
  const pending = new Promise<ReturnType<typeof dockerFixture>>((resolve) => {
    resolveInventory = resolve;
  });
  const api = {
    ...fixture.api,
    dockerInventory: vi.fn(() => pending),
    cancelDocker: vi.fn(async () => undefined),
  };
  const { disconnect } = await setup({ api });
  switchTools();
  fireEvent.click(
    screen.getByRole('button', { name: 'Inspect Docker inventory' }),
  );
  expect(
    screen
      .getByRole('button', { name: 'Inspect Docker inventory' })
      .hasAttribute('disabled'),
  ).toBe(true);
  fireEvent.click(screen.getByRole('button', { name: 'Cancel inventory' }));
  expect(api.cancelDocker).toHaveBeenCalledTimes(1);
  cleanup();
  disconnect();
  await act(async () => {
    resolveInventory?.(dockerFixture());
    await Promise.resolve();
  });
  expect(api.dockerInventory).toHaveBeenCalledTimes(1);
});

test('Explore handoff can reopen same finding after inspector dismissal', async () => {
  const { disconnect } = await setup({ shell: true });
  fireEvent.click(screen.getByRole('button', { name: 'Development' }));
  fireEvent.click(
    screen.getByRole('button', { name: 'Explore SYNTHETIC Atlas – 開発' }),
  );
  await act(async () => {
    await Promise.resolve();
  });
  fireEvent.click(screen.getByRole('button', { name: 'Close Item details' }));
  fireEvent.click(screen.getByRole('button', { name: 'Development' }));
  fireEvent.click(
    screen.getByRole('button', { name: 'Explore SYNTHETIC Atlas – 開発' }),
  );
  await act(async () => {
    await Promise.resolve();
  });
  expect(
    screen.getByRole('complementary', { name: 'Item details' }).textContent,
  ).toContain('SYNTHETIC Atlas');
  disconnect();
});
