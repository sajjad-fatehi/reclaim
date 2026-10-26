import { useState } from 'react';
import type { ReactElement } from 'react';
import { AppShell } from '../shell/AppShell';
import type { Destination } from '../shell/AppShell';
import { Notice } from '../shell/primitives';
import { useScanSession } from './useScanSession';
import { Overview } from './Overview';
import { ScanSettings } from './ScanSettings';

export function ScanApplication({
  connectionStatus,
}: {
  connectionStatus: string;
}): ReactElement {
  const { state, session } = useScanSession();
  const [destination, setDestination] = useState<Destination>('overview');
  const progress = state.snapshot?.progress;
  return (
    <AppShell
      connectionStatus={connectionStatus}
      destination={destination}
      onNavigate={setDestination}
      appearance={{
        preference: state.preferences.appearance,
        persistenceError: false,
        changeAppearance: (appearance) => {
          session.editPreferences({ appearance });
          void session.save();
        },
      }}
      context={
        <span>
          {progress
            ? `Generation ${progress.generation} · ${progress.status}${state.stale ? ' · stale' : ''}`
            : 'Scan not started'}
        </span>
      }
      views={{
        overview: <Overview onInvestigate={() => setDestination('explore')} />,
        settings: (
          <>
            <ScanSettings />
            {state.error && <p role="alert">{state.error}</p>}
          </>
        ),
        explore: state.investigation ? (
          <div className="investigation-handoff">
            <h2>{state.investigation.category}</h2>
            <p>
              {state.investigation.findingIds.length} measured files ·
              generation {state.investigation.generation}
            </p>
            <Notice>
              Explore navigation and shared inspector are coming in the next
              feature. This category context is retained; no additional scan
              starts.
            </Notice>
            <button onClick={() => setDestination('overview')}>
              Back to Overview
            </button>
          </div>
        ) : undefined,
      }}
    />
  );
}
