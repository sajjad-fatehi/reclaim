import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { AppShell } from './AppShell';
import { appearanceKey, readAppearance } from './appearance';

let darkMode = false;
const listeners = new Set<() => void>();
beforeEach((): void => {
  localStorage.clear();
  darkMode = false;
  vi.stubGlobal(
    'matchMedia',
    vi.fn(() => ({
      get matches(): boolean {
        return darkMode;
      },
      addEventListener: (_event: string, listener: () => void): void => {
        listeners.add(listener);
      },
      removeEventListener: (_event: string, listener: () => void): void => {
        listeners.delete(listener);
      },
    })),
  );
});
afterEach((): void => {
  cleanup();
  listeners.clear();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

test('rejects corrupt appearance without touching unrelated settings', (): void => {
  localStorage.setItem('scan-settings', 'unchanged');
  for (const value of ['{"theme":"dark"}', 'DARK', '', 'null']) {
    localStorage.setItem(appearanceKey, value);
    expect(readAppearance()).toBe('system');
  }
  expect(localStorage.getItem('scan-settings')).toBe('unchanged');
});

test('appearance survives remount and System follows live OS changes', (): void => {
  const firstRender = render(<AppShell connectionStatus="Connected" />);
  fireEvent.click(screen.getByRole('button', { name: 'Settings' }));
  fireEvent.click(screen.getByRole('radio', { name: 'Dark' }));
  expect(document.documentElement.dataset.theme).toBe('dark');
  expect(localStorage.getItem(appearanceKey)).toBe('dark');
  firstRender.unmount();
  render(<AppShell connectionStatus="Connected" />);
  expect(document.documentElement.dataset.theme).toBe('dark');
  fireEvent.click(screen.getByRole('button', { name: 'Settings' }));
  fireEvent.click(screen.getByRole('radio', { name: 'System' }));
  expect(document.documentElement.dataset.theme).toBe('light');
  darkMode = true;
  listeners.forEach((listener) => listener());
  expect(document.documentElement.dataset.theme).toBe('dark');
  fireEvent.click(screen.getByRole('radio', { name: 'Light' }));
  listeners.forEach((listener) => listener());
  expect(document.documentElement.dataset.theme).toBe('light');
});

test('unavailable storage keeps session choice and explains failed persistence', (): void => {
  vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
    throw new Error('unavailable');
  });
  vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
    throw new Error('full');
  });
  render(<AppShell connectionStatus="Connected" />);
  fireEvent.click(screen.getByRole('button', { name: 'Settings' }));
  fireEvent.click(screen.getByRole('radio', { name: 'Dark' }));
  expect(document.documentElement.dataset.theme).toBe('dark');
  expect(screen.getByText(/Appearance could not be saved/)).toBeTruthy();
});

test('navigation retains mounted view inputs and scroll containers, with honest fallback', (): void => {
  render(
    <AppShell
      connectionStatus="Connected"
      views={{
        explore: (
          <label>
            Search <input type="search" />
          </label>
        ),
      }}
    />,
  );
  expect(
    screen.getByText(/Storage scanning is not available yet/),
  ).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Explore' }));
  const input = screen.getByRole('searchbox');
  fireEvent.change(input, { target: { value: 'retained query' } });
  const view = input.closest('section');
  if (!view) throw new Error('Missing view');
  view.scrollTop = 150;
  fireEvent.click(screen.getByRole('button', { name: 'Development' }));
  expect(screen.queryByRole('searchbox')).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Explore' }));
  expect(screen.getByRole('searchbox')).toBe(input);
  expect(input).toHaveProperty('value', 'retained query');
  expect(view.scrollTop).toBe(150);
  expect(
    screen
      .getByRole('button', { name: 'Explore' })
      .getAttribute('aria-current'),
  ).toBe('page');
  expect(
    screen.queryByRole('button', { name: /delete|clean now|scan now/i }),
  ).toBeNull();
});
