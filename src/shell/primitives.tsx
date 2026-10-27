import { Info } from '@phosphor-icons/react/Info';
import { X } from '@phosphor-icons/react/X';
import { useEffect, useId, useRef, useState } from 'react';
import type { ReactElement, ReactNode, RefObject } from 'react';
import { usePanelMotion } from './motion';

export function Toolbar({
  children,
  label,
}: {
  children: ReactNode;
  label: string;
}): ReactElement {
  return (
    <div className="toolbar" role="group" aria-label={label}>
      {children}
    </div>
  );
}

export function Notice({ children }: { children: ReactNode }): ReactElement {
  return (
    <div className="notice">
      <Info aria-hidden="true" size={20} />
      <div>{children}</div>
    </div>
  );
}

export function EmptyState({
  icon,
  title,
  children,
}: {
  icon: ReactNode;
  title: string;
  children: ReactNode;
}): ReactElement {
  return (
    <div className="empty-state">
      <div className="empty-icon" aria-hidden="true">
        {icon}
      </div>
      <h2>{title}</h2>
      <div className="empty-description">{children}</div>
    </div>
  );
}

export function ResultTable({
  caption,
  children,
}: {
  caption: string;
  children: ReactNode;
}): ReactElement {
  return (
    <div
      className="table-scroll"
      role="region"
      aria-label={caption}
      tabIndex={0}
    >
      <table className="result-table">
        <caption>{caption}</caption>
        {children}
      </table>
    </div>
  );
}

interface PanelProps {
  title: string;
  open: boolean;
  onClose: () => void;
  children: ReactNode;
}

export function Dialog({
  title,
  open,
  onClose,
  children,
  placement = 'center',
  initialFocus,
}: PanelProps & {
  placement?: 'center' | 'edge';
  initialFocus?: RefObject<HTMLElement | null>;
}): ReactElement {
  const reference = useRef<HTMLDialogElement>(null);
  const headingId = useId();
  usePanelMotion(reference, open, placement === 'edge' ? 'edge' : 'view');
  useEffect(() => {
    const dialog = reference.current;
    if (!dialog) return;
    if (open && !dialog.open) {
      dialog.showModal();
      initialFocus?.current?.focus();
    }
    if (!open && dialog.open) dialog.close();
  }, [open, initialFocus]);
  return (
    <dialog
      className={placement === 'edge' ? 'dialog inspector-sheet' : 'dialog'}
      ref={reference}
      aria-labelledby={headingId}
      onClose={() => {
        if (open) onClose();
      }}
    >
      <header className="panel-header">
        <h2 id={headingId}>{title}</h2>
        <button
          type="button"
          autoFocus
          onClick={onClose}
          aria-label={`Close ${title}`}
          title={`Close ${title}`}
        >
          <X aria-hidden="true" size={20} />
        </button>
      </header>
      {children}
    </dialog>
  );
}

export function Inspector({
  title,
  open,
  onClose,
  children,
}: PanelProps): ReactElement {
  const reference = useRef<HTMLElement>(null);
  const closeReference = useRef<HTMLButtonElement>(null);
  const headingId = useId();
  const [compact, setCompact] = useState(
    () => window.matchMedia('(max-width: 1100px)').matches,
  );
  usePanelMotion(reference, open, 'edge');
  useEffect(() => {
    const query = window.matchMedia('(max-width: 1100px)');
    function updateCompact(): void {
      setCompact(query.matches);
    }
    query.addEventListener('change', updateCompact);
    return () => query.removeEventListener('change', updateCompact);
  }, []);
  useEffect(() => {
    if (!open || compact) return;
    const invoker = document.activeElement;
    closeReference.current?.focus();
    return () => {
      if (invoker instanceof HTMLElement && invoker.isConnected)
        invoker.focus();
    };
  }, [open, compact]);
  if (compact)
    return (
      <Dialog title={title} open={open} onClose={onClose} placement="edge">
        <div className="inspector-content">{children}</div>
      </Dialog>
    );
  return (
    <aside
      className="inspector"
      ref={reference}
      hidden={!open}
      aria-labelledby={headingId}
      onKeyDown={(event) => {
        if (event.key === 'Escape') {
          event.stopPropagation();
          onClose();
        }
      }}
    >
      <header className="panel-header">
        <h2 id={headingId}>{title}</h2>
        <button
          type="button"
          ref={closeReference}
          onClick={onClose}
          aria-label={`Close ${title}`}
          title={`Close ${title}`}
        >
          <X aria-hidden="true" size={20} />
        </button>
      </header>
      <div className="inspector-content">{children}</div>
    </aside>
  );
}
