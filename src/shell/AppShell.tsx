import { ChartPieSlice } from '@phosphor-icons/react/ChartPieSlice';
import { Code } from '@phosphor-icons/react/Code';
import { FolderOpen } from '@phosphor-icons/react/FolderOpen';
import { HardDrives } from '@phosphor-icons/react/HardDrives';
import { ListChecks } from '@phosphor-icons/react/ListChecks';
import { GearSix } from '@phosphor-icons/react/GearSix';
import { Monitor } from '@phosphor-icons/react/Monitor';
import { Moon } from '@phosphor-icons/react/Moon';
import { Sun } from '@phosphor-icons/react/Sun';
import { Warning } from '@phosphor-icons/react/Warning';
import { ArrowRight } from '@phosphor-icons/react/ArrowRight';
import { useLayoutEffect, useRef, useState } from 'react';
import type { ReactElement, ReactNode } from 'react';
import { useAppearance } from './appearance';
import type { AppearanceState } from './appearance';
import { usePanelMotion } from './motion';
import { EmptyState, Notice } from './primitives';
import folderIllustration from '../../assets/illustrations/reclaim-folder.webp';

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
const primaryDestinations = destinations.filter(
  (entry) => entry.id !== 'settings',
);

export type ScanStatusTone = 'idle' | 'active' | 'caution';
export interface ScanStatus {
  label: string;
  tone: ScanStatusTone;
}

export interface AppShellProps {
  views?: Partial<Record<Destination, ReactNode>>;
  scanStatus?: ScanStatus;
  reviewCount?: number;
  connectionStatus: string;
  appearance?: AppearanceState;
  destination?: Destination;
  onNavigate?: (destination: Destination) => void;
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
  const scrollPositions = useRef(
    new Map<HTMLElement, { top: number; left: number }>(),
  );
  useLayoutEffect(() => {
    if (!active) return;
    scrollPositions.current.forEach((position, element) => {
      if (!element.isConnected) {
        scrollPositions.current.delete(element);
        return;
      }
      element.scrollTop = position.top;
      element.scrollLeft = position.left;
    });
  }, [active]);
  usePanelMotion(reference, active);
  return (
    <section
      ref={reference}
      className="view"
      hidden={!active}
      aria-label={title}
      onScrollCapture={(event) => {
        if (!active || !(event.target instanceof HTMLElement)) return;
        scrollPositions.current.set(event.target, {
          top: event.target.scrollTop,
          left: event.target.scrollLeft,
        });
      }}
    >
      {children}
    </section>
  );
}

export function AppShell({
  views = {},
  scanStatus = { label: 'Scan not started', tone: 'idle' },
  reviewCount = 0,
  connectionStatus,
  appearance,
  destination: controlledDestination,
  onNavigate,
}: AppShellProps): ReactElement {
  const [localDestination, setLocalDestination] =
    useState<Destination>('overview');
  const destination = controlledDestination ?? localDestination;
  const setDestination = onNavigate ?? setLocalDestination;
  const { preference, persistenceError, changeAppearance } =
    useAppearance(appearance);
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
      <header className="window-header">
        <div className="brand">
          <span className="brand-symbol" aria-hidden="true" />
          <span>Reclaim</span>
        </div>
        <nav className="navigation-pill" aria-label="Main navigation">
          {primaryDestinations.map(({ id, label, icon: Icon }) => (
            <button
              key={id}
              type="button"
              className="navigation-tab"
              aria-current={destination === id ? 'page' : undefined}
              aria-controls={`view-${id}`}
              onClick={() => setDestination(id)}
            >
              <Icon
                size={18}
                weight={destination === id ? 'fill' : 'regular'}
                aria-hidden="true"
              />
              <span>{label}</span>
              {id === 'review' && reviewCount > 0 && (
                <span className="navigation-count">
                  <span aria-hidden="true">{reviewCount}</span>
                  <span className="visually-hidden">
                    {reviewCount} selected
                  </span>
                </span>
              )}
            </button>
          ))}
        </nav>
        <div className="window-status">
          <span className={`status-chip status-${scanStatus.tone}`}>
            {scanStatus.tone === 'caution' ? (
              <Warning size={16} aria-hidden="true" />
            ) : (
              <HardDrives size={16} aria-hidden="true" />
            )}
            <span>{scanStatus.label}</span>
          </span>
          <button
            type="button"
            className="round-button"
            aria-current={destination === 'settings' ? 'page' : undefined}
            aria-controls="view-settings"
            aria-label="Settings"
            title="Settings"
            onClick={() => setDestination('settings')}
          >
            <GearSix
              size={18}
              weight={destination === 'settings' ? 'fill' : 'regular'}
              aria-hidden="true"
            />
          </button>
        </div>
      </header>
      <main id="workspace" ref={mainReference} tabIndex={-1}>
        <h1 className="visually-hidden">
          {destinations.find((entry) => entry.id === destination)?.label}
        </h1>
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
                    <section className="settings-card appearance-card">
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
                          Appearance could not be saved. Your choice applies
                          until Reclaim closes.
                        </Notice>
                      )}
                    </section>
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
                      <div
                        className={`section-introduction introduction-${id}`}
                      >
                        <EmptyState icon={<Icon size={28} />} title={title}>
                          <p>{description}</p>
                          {id === 'overview' && (
                            <button
                              type="button"
                              className="primary-action"
                              onClick={() => setDestination('settings')}
                            >
                              Personalize appearance{' '}
                              <ArrowRight size={18} aria-hidden="true" />
                            </button>
                          )}
                        </EmptyState>
                        {id === 'overview' && (
                          <img
                            className="folder-illustration"
                            src={folderIllustration}
                            width={1200}
                            height={1200}
                            alt=""
                            aria-hidden="true"
                          />
                        )}
                      </div>
                      <Notice>
                        This view is not available yet. No files are removed
                        here.
                      </Notice>
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
