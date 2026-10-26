import type {
  Finding,
  Location,
  Preferences,
  Progress,
  ScanApi,
  Scope,
} from './contract';
import { emptyPreferences } from './session';

export const fixtureLocation: Location = {
  id: '11111111-1111-4111-8111-111111111111',
  displayPath: '/SYNTHETIC/Projects',
  volume: {
    id: 'fixture',
    measuredAtMs: 1000,
    totalBytes: '1000000',
    usedBytes: '600000',
    freeBytes: '400000',
    availableBytes: '350000',
    metricIssue: null,
  },
};
export function fixtureProgress(
  status: Progress['status'] = 'complete',
  generation = 1,
): Progress {
  return {
    generation,
    status,
    startedAtMs: 1000,
    currentLocation: fixtureLocation.displayPath,
    finishedAtMs: status === 'running' ? null : 2000,
    entries: 2,
    issueCount: 0,
    terminalIssue: null,
    logicalBytes: '5',
    allocatedBytes: '4096',
    volumes: [
      {
        volume: fixtureLocation.volume,
        scopeLogicalBytes: '5',
        scopeAllocatedBytes: '4096',
        unscannedOrUnexplainedBytes: '595904',
      },
    ],
  };
}
export function fixtureFindings(generation = 1): Finding[] {
  return [
    {
      id: `${generation}:0`,
      parentId: null,
      displayName: fixtureLocation.displayPath,
      kind: 'directory',
      logicalBytes: '5',
      allocatedBytes: '4096',
      measuredAtMs: 2000,
      modifiedAtMs: null,
      identity: 'fixture:1',
      aliasOf: null,
      complete: true,
      issue: null,
    },
    {
      id: `${generation}:1`,
      parentId: `${generation}:0`,
      displayName: 'synthetic.txt',
      kind: 'file',
      logicalBytes: '5',
      allocatedBytes: '4096',
      measuredAtMs: 2000,
      modifiedAtMs: null,
      identity: 'fixture:2',
      aliasOf: null,
      complete: true,
      issue: null,
    },
  ];
}
export function createFixtureApi(): ScanApi {
  let preferences: Preferences = {
    ...emptyPreferences,
    roots: [],
    exclusions: [],
  };
  let scope: Scope = { id: 'scope', roots: [fixtureLocation], exclusions: [] };
  let generation = 0;
  return {
    loadPreferences: async () => preferences,
    savePreferences: async (next) => {
      preferences = { ...next, configured: true, corrupt: false };
    },
    locations: async () => ({
      candidates: [fixtureLocation],
      unavailableCount: 0,
    }),
    pickFolder: async () => ({
      ...fixtureLocation,
      id: '22222222-2222-4222-8222-222222222222',
      displayPath: '/SYNTHETIC/Added',
    }),
    register: async (_ids, exclusions) => {
      scope = { ...scope, exclusions };
      return scope;
    },
    start: async () => fixtureProgress('running', ++generation),
    current: async () => ({ scope: null, progress: null }),
    page: async (currentGeneration, offset) => ({
      progress: fixtureProgress('complete', currentGeneration),
      offset,
      nextOffset: null,
      findings: fixtureFindings(currentGeneration),
    }),
    cancel: async (currentGeneration) =>
      fixtureProgress('cancelled', currentGeneration),
  };
}
