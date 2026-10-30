import { useEffect, useRef } from 'react';
import type { ReactElement } from 'react';
import { ScanProvider } from '../scan/context';
import {
  createFixtureApi,
  fixtureFindings,
  fixtureLocation,
  fixtureProgress,
  serveScan,
} from '../scan/fixtures';
import { Review } from './Review';
import { ReviewProvider } from './ReviewProvider';
import { useReview } from './useReview';
import type {
  CleanupApi,
  ItemOutcome,
  Operation,
  Review as ReviewReport,
} from './contract';
import type { Selection } from './contract';

type Scenario = 'success' | 'partial' | 'failed' | 'changed' | 'irreversible';

const selections: Selection[] = [
  { source: 'filesystem', generation: 1, findingId: '1:0' },
  { source: 'filesystem', generation: 1, findingId: '1:1' },
  { source: 'docker', findingId: 'docker:build-cache' },
];
const scanApi = createFixtureApi();
scanApi.current = async () => ({
  scope: { id: 'fixture', roots: [fixtureLocation], exclusions: [] },
  progress: fixtureProgress(),
});
serveScan(scanApi, fixtureProgress(), fixtureFindings());

function buildReview(scenario: Scenario): ReviewReport {
  const targets = [
    ['/SYNTHETIC/atlas-web/target', 'trash'],
    ['/SYNTHETIC/atlas-web/node_modules', 'trash'],
    ['Docker build cache', 'docker_image'],
  ] as const;
  return {
    checkedAtMs: Date.now(),
    planId: 'fixture-plan',
    expiresAtMs: null,
    items: selections.map((selection, index) => {
      const [target, method] = targets[index] ?? targets[0];
      const irreversible = method !== 'trash';
      return {
        selection,
        target,
        logicalBytes: '9600000000',
        allocatedBytes: '9600000000',
        measuredAtMs: Date.now() - 60000,
        artifact: null,
        unavailableReason: irreversible
          ? 'Non-force Docker builder prune. Cached layers rebuild on next build.'
          : 'Regenerable build output. Rebuild restores it.',
        changed: scenario === 'changed' && index === 1,
        method:
          scenario === 'changed' && index === 1
            ? null
            : scenario !== 'irreversible' && irreversible
              ? 'trash'
              : method,
      };
    }),
  };
}

function buildOperation(scenario: Scenario): Operation {
  const outcomes: Record<Scenario, ItemOutcome['outcome'][]> = {
    success: ['removed', 'removed', 'removed'],
    partial: ['removed', 'failed', 'unknown'],
    failed: ['failed', 'failed', 'skipped'],
    changed: ['removed', 'changed', 'removed'],
    irreversible: ['removed', 'removed', 'removed'],
  };
  const explanations: Record<ItemOutcome['outcome'], string> = {
    removed: 'Moved to macOS Trash; physical space not claimed.',
    failed: 'Operation reported an error. Target left as it was found.',
    unknown:
      'Tool did not report a result. Inspect the target before retrying.',
    changed:
      'Target changed since review. Skipped; nothing was removed. Check again.',
    skipped: 'Cancelled before dispatch. Nothing undone.',
    pending: 'Waiting.',
  };
  return {
    id: 'fixture-operation',
    startedAtMs: Date.now() - 4000,
    finishedAtMs: Date.now(),
    volumeObservations: [],
    items: outcomes[scenario].map((outcome, index) => ({
      label:
        ['atlas-web/target', 'atlas-web/node_modules', 'Docker build cache'][
          index
        ] ?? '',
      method: index === 2 ? 'docker_image' : 'trash',
      outcome,
      explanation: explanations[outcome],
    })),
  };
}

function createApi(scenario: Scenario): CleanupApi {
  return {
    review: async () => buildReview(scenario),
    execute: async (planId) => ({
      planId,
      operation: buildOperation(scenario),
      running: false,
      persistenceError: false,
    }),
    status: async () => null,
    cancel: async () => undefined,
    history: async () => ({
      history: { operations: [buildOperation('partial')] },
      unavailable: false,
    }),
    clearHistory: async () => undefined,
  };
}

function Preload(): null {
  const review = useReview();
  const loaded = useRef(false);
  useEffect(() => {
    if (loaded.current || !review) return;
    loaded.current = true;
    selections.forEach((selection) => review.add(selection));
  }, [review]);
  return null;
}

export function CleanupFixture(): ReactElement {
  const scenario = (new URLSearchParams(location.search).get('scenario') ??
    'success') as Scenario;
  return (
    <ScanProvider api={scanApi}>
      <ReviewProvider>
        <Preload />
        <div className="view">
          <Review active api={createApi(scenario)} />
        </div>
      </ReviewProvider>
    </ScanProvider>
  );
}
