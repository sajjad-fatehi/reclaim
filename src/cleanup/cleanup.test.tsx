import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  within,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { ScanContext } from '../scan/useScanSession';
import { ScanSession } from '../scan/session';
import {
  createFixtureApi,
  fixtureLocation,
  fixtureProgress,
  fixtureFindings,
} from '../scan/fixtures';
import { ScanApplication } from '../scan/ScanApplication';
import { parseHistory, parseReview } from './contract';
import type { CleanupApi, Execution, Review, Selection } from './contract';

const disconnects: Array<() => void> = [];
beforeEach(() => {
  Object.defineProperty(HTMLDialogElement.prototype, 'showModal', {
    configurable: true,
    value: function showModal(this: HTMLDialogElement): void {
      this.open = true;
    },
  });
  Object.defineProperty(HTMLDialogElement.prototype, 'close', {
    configurable: true,
    value: function close(this: HTMLDialogElement): void {
      this.open = false;
    },
  });
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
  disconnects.splice(0).forEach((disconnect) => disconnect());
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});
async function settle(): Promise<void> {
  await act(async () => {
    await new Promise<void>((resolve) => setTimeout(resolve, 5));
  });
}
function report(selections: Selection[]): Review {
  return {
    checkedAtMs: 2000,
    planId: null,
    expiresAtMs: null,
    items: selections.map((selection) => ({
      selection,
      target: '/SYNTHETIC/Projects',
      logicalBytes: '5',
      allocatedBytes: '4096',
      measuredAtMs: 2000,
      artifact: null,
      unavailableReason:
        'Trash unavailable: expected identity cannot be enforced.',
      changed: false,
      method: null,
    })),
  };
}
function fixtureApi(): CleanupApi {
  return {
    execute: vi.fn(async () => {
      throw new Error('No executable fixture plan');
    }),
    status: vi.fn(async () => null),
    cancel: vi.fn(async () => undefined),
    review: vi.fn(async (selections) => report(selections)),
    history: vi.fn(async () => ({
      history: { operations: [] },
      unavailable: false,
    })),
    clearHistory: vi.fn(async () => undefined),
  };
}
async function mount(api: CleanupApi): Promise<ScanSession> {
  const native = createFixtureApi();
  native.current = async () => ({
    scope: { id: 'fixture', roots: [fixtureLocation], exclusions: [] },
    progress: fixtureProgress(),
  });
  native.page = async () => ({
    progress: fixtureProgress(),
    offset: 0,
    nextOffset: null,
    findings: fixtureFindings(),
  });
  const session = new ScanSession(native);
  disconnects.push(session.connect());
  await new Promise<void>((resolve) => setTimeout(resolve, 400));
  render(
    <ScanContext.Provider value={session}>
      <ScanApplication connectionStatus="SYNTHETIC test" cleanupApi={api} />
    </ScanContext.Provider>,
  );
  await settle();
  return session;
}
test('explicit selection survives navigation; native unavailability never exposes confirmation or removal', async () => {
  const api = fixtureApi();
  await mount(api);
  const location = screen.getByRole('region', {
    name: 'Review scanned locations',
  });
  fireEvent.click(
    within(location).getByRole('button', { name: 'Review cleanup options' }),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Explore' }));
  fireEvent.click(screen.getByRole('button', { name: /^Review & Clean/ }));
  expect(
    screen.getByRole('heading', { name: /Review selection/ }),
  ).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Check eligibility' }));
  await settle();
  expect(api.review).toHaveBeenCalledWith([
    { source: 'filesystem', generation: 1, findingId: '1:0' },
  ]);
  expect(
    screen.getByText(
      'Trash unavailable: expected identity cannot be enforced.',
    ),
  ).toBeTruthy();
  expect(
    screen.queryByRole('button', {
      name: /confirm|remove selected|execute|recover/i,
    }),
  ).toBeNull();
  expect(screen.getByText(/No recorded operations/)).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Remove from review' }));
  expect(screen.getByText('Choose what to understand')).toBeTruthy();
});
test('double check suppressed, mismatched native response rejected, corrupt history remains visible and clearable', async () => {
  const api = fixtureApi();
  let finish: ((value: Review) => void) | undefined;
  api.review = vi.fn(
    () =>
      new Promise<Review>((resolve) => {
        finish = resolve;
      }),
  );
  api.history = vi.fn(async () => ({ history: null, unavailable: true }));
  await mount(api);
  fireEvent.click(
    within(
      screen.getByRole('region', { name: 'Review scanned locations' }),
    ).getByRole('button', { name: 'Review cleanup options' }),
  );
  fireEvent.click(screen.getByRole('button', { name: /^Review & Clean/ }));
  const button = screen.getByRole('button', { name: 'Check eligibility' });
  fireEvent.click(button);
  fireEvent.click(button);
  expect(api.review).toHaveBeenCalledTimes(1);
  await act(async () => {
    finish?.(report([{ source: 'docker', findingId: 'forged' }]));
  });
  expect(screen.getByRole('alert').textContent).toContain(
    'Eligibility unavailable',
  );
  await settle();
  expect(
    screen.getByText(/History corrupt, inaccessible or invalid/),
  ).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Clear history' }));
  await settle();
  expect(api.clearHistory).toHaveBeenCalledTimes(1);
  expect(screen.getByText(/No recorded operations/)).toBeTruthy();
});
test('malformed and executable-looking transport data cannot become eligibility or history', () => {
  expect(() =>
    parseReview({
      checkedAtMs: 1,
      items: [{ selection: { source: 'shell', command: 'rm' } }],
    }),
  ).toThrow();
  expect(() =>
    parseHistory({ unavailable: true, history: { operations: [] } }),
  ).toThrow();
  expect(() =>
    parseHistory({
      unavailable: false,
      history: { operations: [{ id: 'bad' }] },
    }),
  ).toThrow();
  expect(() =>
    parseHistory({
      unavailable: false,
      history: { operations: Array.from({ length: 51 }, () => ({})) },
    }),
  ).toThrow();
});

test('eligible plan requires separate Cancel-focused exact-target confirmation and shows durable outcomes', async () => {
  const api = fixtureApi();
  api.review = vi.fn(async (selections: Selection[]): Promise<Review> => ({
    ...report(selections),
    planId: 'native-plan',
    expiresAtMs: Date.now() + 120_000,
    items: report(selections).items.map((item) => ({
      ...item,
      method: 'trash',
      unavailableReason:
        'Move exact SYNTHETIC target to macOS Trash. Recover through macOS Trash.',
    })),
  }));
  api.execute = vi.fn(async (planId: string): Promise<Execution> => ({
    planId,
    running: false,
    persistenceError: false,
    operation: {
      id: 'operation',
      startedAtMs: 2000,
      finishedAtMs: 3000,
      volumeObservations: [],
      items: [
        {
          label: '/SYNTHETIC/Projects',
          method: 'trash',
          outcome: 'removed',
          explanation: 'Moved to macOS Trash; physical space not claimed.',
        },
      ],
    },
  }));
  await mount(api);
  fireEvent.click(
    within(
      screen.getByRole('region', { name: 'Review scanned locations' }),
    ).getByRole('button', { name: 'Review cleanup options' }),
  );
  fireEvent.click(screen.getByRole('button', { name: /^Review & Clean/ }));
  fireEvent.click(screen.getByRole('button', { name: 'Check eligibility' }));
  await settle();
  expect(api.execute).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Review confirmation' }));
  const dialog = screen.getByRole('dialog', {
    name: 'Confirm exact cleanup targets',
  });
  const cancel = within(dialog).getByRole('button', {
    name: 'Cancel',
  });
  expect(document.activeElement).toBe(cancel);
  expect(within(dialog).getByText('/SYNTHETIC/Projects')).toBeTruthy();
  fireEvent.click(cancel);
  expect(api.execute).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Review confirmation' }));
  fireEvent.click(
    within(screen.getByRole('dialog')).getByRole('button', {
      name: 'Confirm selected removal',
    }),
  );
  await settle();
  expect(api.execute).toHaveBeenCalledExactlyOnceWith('native-plan');
  expect(screen.getByText('Cleanup outcomes')).toBeTruthy();
  expect(
    screen.getByText('Moved to macOS Trash; physical space not claimed.'),
  ).toBeTruthy();
  expect(
    screen.queryByRole('button', { name: 'Review confirmation' }),
  ).toBeNull();
});

test('irreversible tool confirmation requires acknowledgement; cancellation only requests pending stop', async () => {
  const api = fixtureApi();
  api.review = vi.fn(async (selections: Selection[]): Promise<Review> => ({
    ...report(selections),
    planId: 'git-plan',
    expiresAtMs: Date.now() + 120_000,
    items: report(selections).items.map((item) => ({
      ...item,
      method: 'git_worktree',
    })),
  }));
  let finish:
    ((value: Awaited<ReturnType<CleanupApi['execute']>>) => void) | undefined;
  api.execute = vi.fn(
    () =>
      new Promise<Execution>((resolve) => {
        finish = resolve;
      }),
  );
  await mount(api);
  fireEvent.click(
    within(
      screen.getByRole('region', { name: 'Review scanned locations' }),
    ).getByRole('button', { name: 'Review cleanup options' }),
  );
  fireEvent.click(screen.getByRole('button', { name: /^Review & Clean/ }));
  fireEvent.click(screen.getByRole('button', { name: 'Check eligibility' }));
  await settle();
  fireEvent.click(screen.getByRole('button', { name: 'Review confirmation' }));
  const dialog = screen.getByRole('dialog');
  const confirm = within(dialog).getByRole('button', {
    name: 'Confirm selected removal',
  });
  expect(confirm.hasAttribute('disabled')).toBe(true);
  fireEvent.click(within(dialog).getByRole('checkbox'));
  expect(confirm.hasAttribute('disabled')).toBe(false);
  fireEvent.keyDown(confirm, { key: 'Enter' });
  expect(api.execute).not.toHaveBeenCalled();
  fireEvent.click(confirm);
  await settle();
  fireEvent.click(screen.getByRole('button', { name: 'Cancel pending items' }));
  expect(api.cancel).toHaveBeenCalledTimes(1);
  await act(async () => {
    finish?.({
      planId: 'git-plan',
      running: false,
      persistenceError: true,
      operation: {
        id: 'cancelled',
        startedAtMs: 2000,
        finishedAtMs: 3000,
        volumeObservations: [],
        items: [
          {
            label: '/SYNTHETIC/Projects',
            method: 'git_worktree',
            outcome: 'skipped',
            explanation: 'Cancelled before dispatch. Nothing undone.',
          },
        ],
      },
    });
  });
  expect(screen.getByText(/History persistence failed/)).toBeTruthy();
  expect(
    screen.getByText('Cancelled before dispatch. Nothing undone.'),
  ).toBeTruthy();
});
