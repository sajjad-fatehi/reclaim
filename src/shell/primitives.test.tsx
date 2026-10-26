import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { useRef, useState } from 'react';
import type { ReactElement } from 'react';
import { Dialog, Inspector } from './primitives';
import { usePanelMotion } from './motion';

beforeEach((): void => {
  // JSDOM exposes dialog markup but does not implement native window behavior.
  Object.defineProperties(HTMLDialogElement.prototype, {
    showModal: { configurable: true, writable: true, value: vi.fn() },
    close: { configurable: true, writable: true, value: vi.fn() },
  });
  Object.defineProperty(HTMLElement.prototype, 'animate', {
    configurable: true,
    writable: true,
    value: vi.fn(() => ({ cancel: vi.fn() })),
  });
  vi.stubGlobal(
    'matchMedia',
    vi.fn(() => ({
      matches: false,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    })),
  );
  vi.spyOn(HTMLDialogElement.prototype, 'showModal').mockImplementation(
    function (this: HTMLDialogElement): void {
      this.open = true;
    },
  );
  vi.spyOn(HTMLDialogElement.prototype, 'close').mockImplementation(function (
    this: HTMLDialogElement,
  ): void {
    this.open = false;
  });
});
afterEach((): void => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  Reflect.deleteProperty(HTMLDialogElement.prototype, 'showModal');
  Reflect.deleteProperty(HTMLDialogElement.prototype, 'close');
  Reflect.deleteProperty(HTMLElement.prototype, 'animate');
});

test('controlled dialog uses native modal opening and reports native close', (): void => {
  const onClose = vi.fn();
  const result = render(
    <Dialog title="Fixture dialog" open={false} onClose={onClose}>
      Fixture
    </Dialog>,
  );
  expect(screen.queryByRole('dialog')).toBeNull();
  result.rerender(
    <Dialog title="Fixture dialog" open onClose={onClose}>
      Fixture
    </Dialog>,
  );
  const dialog = screen.getByRole('dialog', { name: 'Fixture dialog' });
  expect(HTMLDialogElement.prototype.showModal).toHaveBeenCalledOnce();
  fireEvent(dialog, new Event('close'));
  expect(onClose).toHaveBeenCalledOnce();
});

test('wide inspector supports Escape and returns focus to its connected invoker', (): void => {
  function Fixture(): ReactElement {
    const [open, setOpen] = useState(false);
    return (
      <>
        <button onClick={() => setOpen(true)}>Inspect</button>
        <Inspector
          title="Fixture inspector"
          open={open}
          onClose={() => setOpen(false)}
        >
          Details
        </Inspector>
      </>
    );
  }
  render(<Fixture />);
  const invoker = screen.getByRole('button', { name: 'Inspect' });
  invoker.focus();
  fireEvent.click(invoker);
  expect(document.activeElement).toBe(
    screen.getByRole('button', { name: 'Close Fixture inspector' }),
  );
  fireEvent.keyDown(document.activeElement ?? document.body, { key: 'Escape' });
  expect(screen.queryByRole('complementary')).toBeNull();
  expect(document.activeElement).toBe(invoker);
});

test('motion skips launch, cancels on interruption and reduced-motion changes', (): void => {
  const cancel = vi.fn();
  const animate = vi.fn(() => ({ cancel }));
  let reduceMotion = false;
  const listeners = new Set<() => void>();
  vi.stubGlobal(
    'matchMedia',
    vi.fn(() => ({
      get matches(): boolean {
        return reduceMotion;
      },
      addEventListener: (_event: string, callback: () => void): void => {
        listeners.add(callback);
      },
      removeEventListener: (_event: string, callback: () => void): void => {
        listeners.delete(callback);
      },
    })),
  );
  vi.stubGlobal('getComputedStyle', () => ({
    getPropertyValue: (property: string): string =>
      property === '--ease-out' ? 'ease-out' : '200ms',
  }));
  Object.defineProperty(HTMLElement.prototype, 'animate', {
    configurable: true,
    writable: true,
    value: animate,
  });
  function Fixture({ visible }: { visible: boolean }): ReactElement {
    const reference = useRef<HTMLDivElement>(null);
    usePanelMotion(reference, visible);
    return (
      <div ref={reference} hidden={!visible}>
        Motion fixture
      </div>
    );
  }
  const result = render(<Fixture visible />);
  expect(animate).not.toHaveBeenCalled();
  result.rerender(<Fixture visible={false} />);
  result.rerender(<Fixture visible />);
  expect(animate).toHaveBeenCalledOnce();
  reduceMotion = true;
  listeners.forEach((listener) => listener());
  expect(cancel).toHaveBeenCalledOnce();
  result.rerender(<Fixture visible={false} />);
  result.rerender(<Fixture visible />);
  expect(animate).toHaveBeenCalledOnce();
});
