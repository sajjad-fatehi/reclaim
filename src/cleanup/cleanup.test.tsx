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
import type { CleanupApi, Review, Selection } from './contract';

const disconnects: Array<() => void> = [];
beforeEach(() => {
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
    })),
  };
}
function fixtureApi(): CleanupApi {
  return {
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
  fireEvent.click(screen.getByRole('button', { name: 'Review & Clean' }));
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
  fireEvent.click(screen.getByRole('button', { name: 'Review & Clean' }));
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
