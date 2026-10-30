import type { ReactElement } from 'react';
import { appearanceKey } from '../shell/appearance';
import type { AccessApi, FullDiskAccess } from '../access/contract';
import { ScanProvider } from './context';
import { ScanApplication } from './ScanApplication';
import {
  createFixtureApi,
  fixtureLocation,
  fixtureProgress,
  serveScan,
} from './fixtures';
import type {
  CategoryName,
  CategoryTotal,
  Finding,
  Progress,
  ScanApi,
  TopFile,
} from './contract';

const gibibyte = 1024n ** 3n;
const fixtureVolume = {
  ...fixtureLocation.volume,
  totalBytes: String(812n * gibibyte),
  usedBytes: String(630n * gibibyte),
  freeBytes: String(182n * gibibyte),
  availableBytes: String(170n * gibibyte),
};
const fileGroups: Array<[string, string, bigint[]]> = [
  ['mov', 'Capture', [38n, 21n, 9n, 4n, 2n]],
  ['pdf', 'Report', [6n, 3n, 2n, 1n]],
  ['zip', 'Backup', [29n, 14n, 5n]],
  ['bin', 'Cache', [11n, 7n, 3n, 2n, 1n, 1n]],
];

function fixtureFindingSet(): Finding[] {
  const root: Finding = {
    id: '1:0',
    parentId: null,
    displayName: '/SYNTHETIC/Projects',
    kind: 'directory',
    logicalBytes: '0',
    allocatedBytes: '0',
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
  };
  const files = fileGroups.flatMap(([extension, prefix, sizes]) =>
    sizes.map((size, position): Finding => ({
      ...root,
      id: `1:${extension}${position}`,
      parentId: root.id,
      displayName: `SYNTHETIC-${prefix}-${position + 1}.${extension}`,
      kind: 'file',
      logicalBytes: String(size * gibibyte - 4096n),
      allocatedBytes: String(size * gibibyte),
      identity: `fixture:${extension}${position}`,
    })),
  );
  return [root, ...files];
}

const categoryByExtension: Record<string, CategoryName> = {
  mov: 'Media',
  pdf: 'Documents',
  zip: 'Archives',
  bin: 'Other files',
};
const categoryOrder: CategoryName[] = [
  'Documents',
  'Media',
  'Archives',
  'Other files',
];

function categoryOf(finding: Finding): CategoryName {
  const extension = finding.displayName.split('.').pop() ?? '';
  return categoryByExtension[extension] ?? 'Other files';
}

function categoriesFor(findings: Finding[], volumeId: string): CategoryTotal[] {
  const files = findings.filter(
    (finding) =>
      finding.kind === 'file' && finding.identity?.startsWith(`${volumeId}:`),
  );
  return categoryOrder.map((name) => {
    const members = files.filter((file) => categoryOf(file) === name);
    const sum = (pick: (file: Finding) => string): string =>
      String(members.reduce((total, file) => total + BigInt(pick(file)), 0n));
    return {
      name,
      logicalBytes: sum((file) => file.logicalBytes),
      allocatedBytes: sum((file) => file.allocatedBytes),
      count: members.length,
    };
  });
}

function topFilesFor(findings: Finding[]): TopFile[] {
  return findings
    .filter((finding) => finding.kind === 'file')
    .map((finding) => ({
      category: categoryOf(finding),
      displayName: finding.displayName,
      displayPath: `${fixtureLocation.displayPath}/${finding.displayName}`,
      logicalBytes: finding.logicalBytes,
      allocatedBytes: finding.allocatedBytes,
      modifiedAtMs: null,
      identity: finding.identity ?? finding.id,
      findingId: finding.id,
    }));
}

function measuredBytes(findings: Finding[]): bigint {
  return findings
    .filter((finding) => finding.kind === 'file')
    .reduce((total, finding) => total + BigInt(finding.allocatedBytes), 0n);
}

function progressFor(
  status: Progress['status'],
  findings: Finding[],
): Progress {
  const measured = measuredBytes(findings);
  const unscanned =
    BigInt(fixtureVolume.totalBytes) -
    BigInt(fixtureVolume.freeBytes) -
    measured;
  return {
    ...fixtureProgress(status),
    startedAtMs: Date.now() - 94_000,
    finishedAtMs: status === 'running' ? null : Date.now() - 2_000,
    entries: findings.length * 2431,
    issueCount: status === 'complete' ? 0 : 3,
    logicalBytes: String(measured),
    allocatedBytes: String(measured),
    volumes: [
      {
        volume: fixtureVolume,
        scopeLogicalBytes: String(measured),
        scopeAllocatedBytes: String(measured),
        categories: categoriesFor(findings, fixtureVolume.id),
        unscannedOrUnexplainedBytes: String(unscanned),
      },
    ],
  };
}

const secondVolume = {
  ...fixtureVolume,
  id: 'fixture2',
  totalBytes: String(500n * gibibyte),
  usedBytes: String(300n * gibibyte),
  freeBytes: String(200n * gibibyte),
  availableBytes: String(190n * gibibyte),
};

function secondVolumeFindings(): Finding[] {
  const [root] = fixtureFindingSet();
  return [18n, 12n, 7n].map((size, position): Finding => ({
    ...root!,
    id: `1:v${position}`,
    parentId: root!.id,
    displayName: `SYNTHETIC-Volume-${position + 1}.mov`,
    kind: 'file',
    logicalBytes: String(size * gibibyte - 4096n),
    allocatedBytes: String(size * gibibyte),
    identity: `fixture2:v${position}`,
  }));
}

const issueFindings: Finding[] = [
  {
    ...fixtureFindingSet()[0]!,
    id: '1:i0',
    parentId: '1:0',
    displayName: 'Containers',
    complete: false,
    issue: 'permission_denied',
    identity: 'fixture:i0',
  },
  {
    ...fixtureFindingSet()[0]!,
    id: '1:i1',
    parentId: '1:0',
    displayName: 'Developer',
    complete: false,
    issue: 'permission_denied',
    identity: 'fixture:i1',
  },
];

type FixtureState =
  | 'empty'
  | 'running'
  | 'complete'
  | 'partial'
  | 'cancelled'
  | 'failed'
  | 'multivolume';

function createOverviewApi(state: FixtureState): ScanApi {
  const api = createFixtureApi();
  if (state === 'empty') return api;
  const multivolume = state === 'multivolume';
  const complete = state === 'complete' || multivolume;
  const status: Progress['status'] = multivolume ? 'complete' : state;
  const findings = multivolume
    ? [...fixtureFindingSet(), ...secondVolumeFindings()]
    : complete
      ? fixtureFindingSet()
      : [...fixtureFindingSet(), ...issueFindings];
  const progress = progressFor(status, findings);
  if (multivolume) {
    const secondMeasured = measuredBytes(secondVolumeFindings());
    const firstMeasured = measuredBytes(fixtureFindingSet());
    progress.volumes = [
      {
        volume: fixtureVolume,
        scopeLogicalBytes: String(firstMeasured),
        scopeAllocatedBytes: String(firstMeasured),
        categories: categoriesFor(fixtureFindingSet(), fixtureVolume.id),
        unscannedOrUnexplainedBytes: String(
          BigInt(fixtureVolume.totalBytes) -
            BigInt(fixtureVolume.freeBytes) -
            firstMeasured,
        ),
      },
      {
        volume: secondVolume,
        scopeLogicalBytes: String(secondMeasured),
        scopeAllocatedBytes: String(secondMeasured),
        categories: categoriesFor(secondVolumeFindings(), secondVolume.id),
        unscannedOrUnexplainedBytes: String(
          BigInt(secondVolume.totalBytes) -
            BigInt(secondVolume.freeBytes) -
            secondMeasured,
        ),
      },
    ];
  }
  api.current = async () => ({
    scope: { id: 'fixture', roots: [fixtureLocation], exclusions: [] },
    progress,
  });
  const topFiles = state === 'running' ? [] : topFilesFor(findings);
  serveScan(api, progress, state === 'running' ? [] : findings, topFiles);
  return api;
}

export function OverviewFixture(): ReactElement {
  const theme = new URLSearchParams(location.search).get('theme');
  if (theme === 'light' || theme === 'dark')
    localStorage.setItem(appearanceKey, theme);
  const requested = new URLSearchParams(location.search).get('state');
  const state: FixtureState =
    (
      [
        'empty',
        'running',
        'complete',
        'partial',
        'cancelled',
        'failed',
        'multivolume',
      ] as const
    ).find((candidate) => candidate === requested) ?? 'partial';
  const requestedAccess = new URLSearchParams(location.search).get('access');
  const access: FullDiskAccess =
    (['granted', 'not_granted', 'unknown'] as const).find(
      (candidate) => candidate === requestedAccess,
    ) ?? 'not_granted';
  const accessApi: AccessApi = {
    status: async () => access,
    openSettings: async () => undefined,
  };
  return (
    <ScanProvider api={createOverviewApi(state)}>
      <ScanApplication
        connectionStatus="SYNTHETIC Overview fixture · browser presentation only"
        accessApi={accessApi}
      />
    </ScanProvider>
  );
}
