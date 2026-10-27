import { useState } from 'react';
import type { ReactElement } from 'react';
import { AppShell } from '../shell/AppShell';
import type { Destination, ScanStatus } from '../shell/AppShell';
import { Explore } from '../explore/Explore';
import { Development } from '../development/Development';
import type { DevelopmentApi } from '../development/contract';
import type { InspectionApi } from '../explore/contract';
import { useScanSession } from './useScanSession';
import type { ScanState } from './session';
import { Overview } from './Overview';
import { ScanSettings } from './ScanSettings';
import { ReviewProvider } from '../cleanup/ReviewProvider';
import { Review } from '../cleanup/Review';
import { useReview } from '../cleanup/useReview';
import type { CleanupApi } from '../cleanup/contract';

export function ScanApplication({
  connectionStatus,
  inspectionApi,
  developmentApi,
  cleanupApi,
}: {
  connectionStatus: string;
  inspectionApi?: InspectionApi | undefined;
  developmentApi?: DevelopmentApi | undefined;
  cleanupApi?: CleanupApi | undefined;
}): ReactElement {
  return (
    <ReviewProvider>
      <ScanApplicationContent
        connectionStatus={connectionStatus}
        inspectionApi={inspectionApi}
        developmentApi={developmentApi}
        cleanupApi={cleanupApi}
      />
    </ReviewProvider>
  );
}

function ScanApplicationContent({
  connectionStatus,
  inspectionApi,
  developmentApi,
  cleanupApi,
}: {
  connectionStatus: string;
  inspectionApi?: InspectionApi | undefined;
  developmentApi?: DevelopmentApi | undefined;
  cleanupApi?: CleanupApi | undefined;
}): ReactElement {
  const { state, session } = useScanSession();
  const review = useReview();
  const [destination, setDestination] = useState<Destination>('overview');
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
      scanStatus={describeScanStatus(state)}
      reviewCount={review?.selections.length ?? 0}
      views={{
        review: <Review active={destination === 'review'} api={cleanupApi} />,
        overview: <Overview onInvestigate={() => setDestination('explore')} />,
        settings: (
          <>
            <ScanSettings />
            {state.error && <p role="alert">{state.error}</p>}
          </>
        ),
        development: (
          <Development
            active={destination === 'development'}
            api={developmentApi}
            inspectionApi={inspectionApi}
            onOverview={() => setDestination('overview')}
            onExplore={(findingId) => {
              if (
                !state.snapshot ||
                !state.snapshot.findings.some(
                  (finding) => finding.id === findingId,
                )
              )
                return;
              session.investigate({
                generation: state.snapshot.progress.generation,
                category: `Development · ${findingId}`,
                findingIds: [findingId],
                selectedId: findingId,
              });
              setDestination('explore');
            }}
          />
        ),
        explore: (
          <Explore
            active={destination === 'explore'}
            api={inspectionApi}
            onOverview={() => setDestination('overview')}
          />
        ),
      }}
    />
  );
}

const statusLabels = {
  running: 'Scanning',
  complete: 'Scan complete',
  partial: 'Partial coverage',
  cancelled: 'Scan cancelled',
  failed: 'Scan failed',
} as const;

function describeScanStatus(state: ScanState): ScanStatus {
  const progress = state.snapshot?.progress;
  if (!progress) return { label: 'Scan not started', tone: 'idle' };
  const label = `${statusLabels[progress.status]}${state.stale ? ' · stale' : ''}`;
  if (progress.status === 'running') return { label, tone: 'active' };
  if (progress.status === 'complete' && !state.stale)
    return { label, tone: 'idle' };
  return { label, tone: 'caution' };
}
