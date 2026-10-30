import type {
  CategoryName,
  CategoryTotal,
  Finding,
  Location,
  Preferences,
  Progress,
  ScanApi,
  Scope,
  Summary,
  TopFile,
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
export function fixtureCategories(
  documentsAllocated = '4096',
): CategoryTotal[] {
  const names: CategoryName[] = [
    'Documents',
    'Media',
    'Archives',
    'Other files',
  ];
  return names.map((name) => ({
    name,
    logicalBytes: name === 'Documents' ? '5' : '0',
    allocatedBytes: name === 'Documents' ? documentsAllocated : '0',
    count: name === 'Documents' ? 1 : 0,
  }));
}
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
    retention: { nodes: 2, thresholdBytes: '1048576', budgetReached: false },
    sharingUntracked: false,
    volumes: [
      {
        volume: fixtureLocation.volume,
        scopeLogicalBytes: '5',
        scopeAllocatedBytes: '4096',
        categories: fixtureCategories(),
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
      foldedItems: 0,
      foldedLogicalBytes: '0',
      foldedAllocatedBytes: '0',
      descendantItems: 0,
      detail: 'full',
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
      foldedItems: 0,
      foldedLogicalBytes: '0',
      foldedAllocatedBytes: '0',
      descendantItems: 0,
      detail: 'full',
    },
  ];
}
export function fixtureTopFiles(generation = 1): TopFile[] {
  return [
    {
      category: 'Documents',
      displayName: 'synthetic.txt',
      displayPath: `${fixtureLocation.displayPath}/synthetic.txt`,
      logicalBytes: '5',
      allocatedBytes: '4096',
      modifiedAtMs: null,
      identity: 'fixture:2',
      findingId: `${generation}:1`,
    },
  ];
}
/** Splits flat findings into the summary shape: roots, top level and issue groups. */
export function summaryOf(
  progress: Progress,
  findings: Finding[],
  topFiles: TopFile[] = fixtureTopFiles(progress.generation),
): Summary {
  const groups = new Map<string, Finding[]>();
  for (const finding of findings)
    if (finding.issue !== null)
      groups.set(finding.issue, [
        ...(groups.get(finding.issue) ?? []),
        finding,
      ]);
  return {
    generation: progress.generation,
    progress,
    roots: findings.filter((finding) => finding.parentId === null),
    topLevel: findings.filter((finding) => finding.parentId !== null),
    topFiles,
    issues: [...groups].map(([issue, examples]) => ({
      issue,
      count: examples.length,
      examples,
    })),
  };
}
/** Makes the fixture API report a fixed terminal progress and its summary. */
export function serveScan(
  api: ScanApi,
  progress: Progress,
  findings: Finding[],
  topFiles?: TopFile[],
): void {
  api.status = async () => progress;
  api.summary = async () => summaryOf(progress, findings, topFiles);
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
    status: async (currentGeneration) =>
      fixtureProgress('complete', currentGeneration),
    summary: async (currentGeneration) =>
      summaryOf(
        fixtureProgress('complete', currentGeneration),
        fixtureFindings(currentGeneration),
      ),
    children: async (currentGeneration, parentId, _cursor, sort) => ({
      generation: currentGeneration,
      parentId,
      sort,
      total: 0,
      nextCursor: null,
      findings: [],
      folded: null,
    }),
    search: async (currentGeneration) => ({
      generation: currentGeneration,
      total: 0,
      findings: [],
    }),
    findings: async (currentGeneration, ids) =>
      fixtureFindings(currentGeneration).filter((finding) =>
        ids.includes(finding.id),
      ),
    cancel: async (currentGeneration) =>
      fixtureProgress('cancelled', currentGeneration),
  };
}
