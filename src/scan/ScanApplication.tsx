import { useState } from 'react';
import type { ReactElement } from 'react';
import { AppShell } from '../shell/AppShell';
import type { Destination } from '../shell/AppShell';
import { Explore } from '../explore/Explore';
import { Development } from '../development/Development';
import type { DevelopmentApi } from '../development/contract';
import type { InspectionApi } from '../explore/contract';
import { useScanSession } from './useScanSession';
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
          {!!review?.selections.length && (
            <button onClick={() => setDestination('review')}>
              Review selection ({review.selections.length})
            </button>
          )}
        </span>
      }
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
