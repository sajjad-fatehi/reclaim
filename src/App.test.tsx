import { cleanup, render, screen } from '@testing-library/react';
import { getName, getVersion } from '@tauri-apps/api/app';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { App } from './App';

vi.mock('@tauri-apps/api/app', () => ({
  getName: vi.fn(),
  getVersion: vi.fn(),
}));

beforeEach((): void => {
  vi.stubGlobal(
    'matchMedia',
    vi.fn(() => ({
      matches: false,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    })),
  );
});

afterEach((): void => {
  vi.unstubAllGlobals();
  cleanup();
  vi.resetAllMocks();
});

test('announces metadata returned by the desktop runtime', async (): Promise<void> => {
  vi.mocked(getName).mockResolvedValue('Reclaim');
  vi.mocked(getVersion).mockResolvedValue('0.2.3');

  render(<App />);
  expect(
    screen.getByText('Connecting to the desktop app…').getAttribute('role'),
  ).toBe('status');
  await screen.findByText('Reclaim 0.2.3 · Desktop connected');
});

test('shows a useful failure without exposing native error details', async (): Promise<void> => {
  vi.mocked(getName).mockRejectedValue(new Error('private native details'));
  vi.mocked(getVersion).mockResolvedValue('0.2.3');

  render(<App />);
  await screen.findByText(
    'Desktop connection unavailable. Close and reopen Reclaim to try again.',
  );
  expect(screen.queryByText(/private native details/)).toBeNull();
});
