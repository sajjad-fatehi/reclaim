import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
} from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { AccessGuidance } from './AccessGuidance';
import { AccessProvider } from './context';
import { decodeAccessStatus } from './contract';
import type { AccessApi, FullDiskAccess } from './contract';
import { Overview } from '../scan/Overview';
import { ScanProvider } from '../scan/context';
import {
  createFixtureApi,
  fixtureFindings,
  fixtureLocation,
  fixtureProgress,
  serveScan,
} from '../scan/fixtures';

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
});

function accessApi(initial: FullDiskAccess): {
  api: AccessApi;
  status: ReturnType<typeof vi.fn>;
  openSettings: ReturnType<typeof vi.fn>;
  set: (next: FullDiskAccess) => void;
} {
  let current = initial;
  const status = vi.fn(async () => current);
  const openSettings = vi.fn(async () => undefined);
  return {
    api: { status, openSettings },
    status,
    openSettings,
    set: (next) => {
      current = next;
    },
  };
}

async function renderGuidance(
  api: AccessApi,
  context: 'first-run' | 'settings' | 'denied' = 'first-run',
): Promise<void> {
  render(
    <AccessProvider api={api}>
      <AccessGuidance context={context} />
    </AccessProvider>,
  );
  await act(async () => undefined);
}

test('not granted shows the card and opens System Settings on request', async () => {
  const { api, openSettings } = accessApi('not_granted');
  await renderGuidance(api);
  expect(
    screen.getByRole('heading', {
      name: 'Allow Full Disk Access for fuller results',
    }),
  ).toBeTruthy();
  expect(screen.getByText(/only measures sizes, read-only/)).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Open System Settings' }));
  expect(openSettings).toHaveBeenCalledTimes(1);
});

test('unknown shows softer guidance', async () => {
  await renderGuidance(accessApi('unknown').api);
  expect(
    screen.getByRole('heading', { name: 'Full Disk Access status is unknown' }),
  ).toBeTruthy();
});

test('granted hides the card except for a status line in Settings', async () => {
  await renderGuidance(accessApi('granted').api, 'first-run');
  expect(
    screen.queryByRole('button', { name: 'Open System Settings' }),
  ).toBeNull();
  expect(screen.queryByRole('status')).toBeNull();
  cleanup();
  await renderGuidance(accessApi('granted').api, 'settings');
  expect(screen.getByRole('status').textContent).toContain(
    'Full Disk Access is granted',
  );
});

test('nothing renders before the first status arrives or outside a provider', () => {
  const { container } = render(<AccessGuidance context="first-run" />);
  expect(container.innerHTML).toBe('');
});

test('window focus re-checks and the card disappears once access is granted', async () => {
  const fake = accessApi('not_granted');
  await renderGuidance(fake.api);
  expect(
    screen.getByRole('button', { name: 'Open System Settings' }),
  ).toBeTruthy();
  fake.set('granted');
  await act(async () => {
    window.dispatchEvent(new Event('focus'));
  });
  expect(fake.status).toHaveBeenCalledTimes(2);
  expect(
    screen.queryByRole('button', { name: 'Open System Settings' }),
  ).toBeNull();
});

test('a failed status check degrades to unknown and a failed opener explains the manual path', async () => {
  const failing: AccessApi = {
    status: async () => {
      throw new Error('boom');
    },
    openSettings: async () => {
      throw new Error('boom');
    },
  };
  await renderGuidance(failing);
  fireEvent.click(screen.getByRole('button', { name: 'Open System Settings' }));
  await act(async () => undefined);
  expect(screen.getByRole('alert').textContent).toContain('Full Disk Access');
});

test('decoder rejects malformed transport as unknown', () => {
  expect(decodeAccessStatus({ fullDiskAccess: 'granted' })).toBe('granted');
  expect(decodeAccessStatus({ fullDiskAccess: 'not_granted' })).toBe(
    'not_granted',
  );
  expect(decodeAccessStatus({ fullDiskAccess: 'maybe' })).toBe('unknown');
  expect(decodeAccessStatus(null)).toBe('unknown');
  expect(decodeAccessStatus('granted')).toBe('unknown');
});

test('Overview lists macOS-denied locations with the same fix action', async () => {
  const scanApi = createFixtureApi();
  const findings = fixtureFindings();
  const [root] = findings;
  const deniedFinding = {
    ...root!,
    id: '1:denied-1',
    parentId: root!.id,
    displayName: 'SYNTHETIC-Protected',
    complete: false,
    issue: 'permission_denied',
  };
  const progress = { ...fixtureProgress('partial'), issueCount: 1 };
  scanApi.current = async () => ({
    scope: { id: 'fixture', roots: [fixtureLocation], exclusions: [] },
    progress,
  });
  serveScan(scanApi, progress, [...findings, deniedFinding]);
  vi.useFakeTimers();
  const fake = accessApi('not_granted');
  render(
    <ScanProvider api={scanApi}>
      <AccessProvider api={fake.api}>
        <Overview onInvestigate={() => undefined} />
      </AccessProvider>
    </ScanProvider>,
  );
  await act(async () => vi.advanceTimersByTimeAsync(2000));
  expect(screen.getByLabelText('Locations macOS denied').textContent).toContain(
    'SYNTHETIC-Protected',
  );
  const buttons = screen.getAllByRole('button', {
    name: 'Open System Settings',
  });
  expect(buttons.length).toBeGreaterThanOrEqual(2);
  fireEvent.click(buttons[buttons.length - 1]!);
  expect(fake.openSettings).toHaveBeenCalled();
});
