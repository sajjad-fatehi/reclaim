import { ChartPieSlice } from '@phosphor-icons/react/ChartPieSlice';
import { Code } from '@phosphor-icons/react/Code';
import { FolderOpen } from '@phosphor-icons/react/FolderOpen';
import { HardDrives } from '@phosphor-icons/react/HardDrives';
import { ListChecks } from '@phosphor-icons/react/ListChecks';
import { GearSix } from '@phosphor-icons/react/GearSix';
import { Monitor } from '@phosphor-icons/react/Monitor';
import { Moon } from '@phosphor-icons/react/Moon';
import { Sun } from '@phosphor-icons/react/Sun';
import { useRef, useState } from 'react';
import type { ReactElement, ReactNode } from 'react';
import { useAppearance } from './appearance';
import { usePanelMotion } from './motion';
import { EmptyState, Notice } from './primitives';

const destinations = [
  {
    id: 'overview',
    label: 'Overview',
    icon: ChartPieSlice,
    title: 'Understand your space',
    description:
      'Overview will connect measured disk usage to places worth investigating. Storage scanning is not available yet.',
  },
  {
    id: 'explore',
    label: 'Explore',
    icon: FolderOpen,
    title: 'Follow the space',
    description:
      'Explore will let you inspect folders and their measured usage. File browsing is not available yet.',
  },
  {
    id: 'development',
    label: 'Development',
    icon: Code,
    title: 'See what belongs where',
    description:
      'Development will connect artifacts to their projects and tools. Artifact recognition is not available yet.',
  },
  {
    id: 'review',
    label: 'Review & Clean',
    icon: ListChecks,
    title: 'Understand before removing',
    description:
      'Review will explain selected items and removal consequences. Cleanup is not available yet.',
  },
  {
    id: 'settings',
    label: 'Settings',
    icon: GearSix,
    title: '',
    description: '',
  },
] as const;
export type Destination = (typeof destinations)[number]['id'];

export interface AppShellProps {
  views?: Partial<Record<Destination, ReactNode>>;
  context?: ReactNode;
  connectionStatus: string;
}

function ViewSlot({
  active,
  title,
  children,
}: {
  active: boolean;
  title: string;
  children: ReactNode;
}): ReactElement {
  const reference = useRef<HTMLElement>(null);
  usePanelMotion(reference, active);
  return (
    <section
      ref={reference}
      className="view"
      hidden={!active}
      aria-label={title}
    >
      {children}
    </section>
  );
}

export function AppShell({
  views = {},
  context,
  connectionStatus,
}: AppShellProps): ReactElement {
  const [destination, setDestination] = useState<Destination>('overview');
  const { preference, persistenceError, changeAppearance } = useAppearance();
  const mainReference = useRef<HTMLElement>(null);

  return (
    <div className="app-shell" data-destination={destination}>
      <a
        className="skip-link"
        href="#workspace"
        onClick={() => mainReference.current?.focus()}
      >
        Skip to content
      </a>
      <aside className="navigation-rail" aria-label="Application sidebar">
        <div className="brand">
          <HardDrives size={24} aria-hidden="true" />
          <span>Reclaim</span>
        </div>
        <nav aria-label="Main navigation">
          <div className="navigation-indicator" aria-hidden="true" />
          {destinations.map(({ id, label, icon: Icon }) => (
            <button
              key={id}
              type="button"
              className={`navigation-item navigation-${id}`}
              aria-current={destination === id ? 'page' : undefined}
              aria-controls={`view-${id}`}
              onClick={() => setDestination(id)}
            >
              <Icon
                size={20}
                weight={destination === id ? 'fill' : 'regular'}
                aria-hidden="true"
              />
              <span>{label}</span>
            </button>
          ))}
        </nav>
        <p className="sidebar-note">
          Local by design.
          <br />
          Your files stay yours.
        </p>
      </aside>
      <main id="workspace" ref={mainReference} tabIndex={-1}>
        <header className="workspace-header">
          <h1>
            {destinations.find((entry) => entry.id === destination)?.label}
          </h1>
          <div className="scan-context">
            {context ?? (
              <>
                <HardDrives size={20} aria-hidden="true" />
                <span>
                  No volume selected{' '}
                  <span className="context-detail">· Scan not started</span>
                </span>
              </>
            )}
          </div>
        </header>
        <div className="view-stack">
          {destinations.map(({ id, label, icon: Icon, title, description }) => (
            <div
              key={id}
              id={`view-${id}`}
              className="view-host"
              hidden={destination !== id}
            >
              <ViewSlot active={destination === id} title={label}>
                {id === 'settings' ? (
                  <div className="settings-content">
                    <h2>Appearance</h2>
                    <p>Choose how Reclaim looks on this Mac.</p>
                    <fieldset className="appearance-options">
                      <legend className="visually-hidden">Appearance</legend>
                      {(
                        [
                          { value: 'system', label: 'System', icon: Monitor },
                          { value: 'light', label: 'Light', icon: Sun },
                          { value: 'dark', label: 'Dark', icon: Moon },
                        ] as const
                      ).map(
                        ({
                          value,
                          label: appearanceLabel,
                          icon: AppearanceIcon,
                        }) => (
                          <label key={value} className="appearance-option">
                            <input
                              type="radio"
                              name="appearance"
                              value={value}
                              checked={preference === value}
                              onChange={() => changeAppearance(value)}
                            />
                            <AppearanceIcon size={20} aria-hidden="true" />
                            <span>{appearanceLabel}</span>
                          </label>
                        ),
                      )}
                    </fieldset>
                    <p className="secondary">
                      System follows your Mac’s appearance automatically.
                    </p>
                    {persistenceError && (
                      <Notice>
                        Appearance could not be saved. Your choice applies until
                        Reclaim closes.
                      </Notice>
                    )}
                    {views.settings}
                    {!views.settings && (
                      <div className="settings-future">
                        <h2>Scan settings</h2>
                        <p>
                          Scan locations, exclusions and permissions will be
                          available with scanning.
                        </p>
                      </div>
                    )}
                  </div>
                ) : (
                  (views[id] ?? (
                    <>
                      <EmptyState icon={<Icon size={40} />} title={title}>
                        <p>{description}</p>
                      </EmptyState>
                      <Notice>No files are scanned or changed.</Notice>
                    </>
                  ))
                )}
              </ViewSlot>
            </div>
          ))}
        </div>
        <footer className="connection-status">
          <p role="status">{connectionStatus}</p>
        </footer>
      </main>
    </div>
  );
}
