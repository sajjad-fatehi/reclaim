import type { ReactElement } from 'react';
import { appearanceKey } from '../shell/appearance';
import { ScanProvider } from './context';
import { ScanApplication } from './ScanApplication';
import { createFixtureApi, fixtureLocation, fixtureProgress } from './fixtures';
import type { Finding, Progress, ScanApi } from './contract';

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
        unscannedOrUnexplainedBytes: String(unscanned),
      },
    ],
  };
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
  'empty' | 'running' | 'complete' | 'partial' | 'cancelled' | 'failed';

function createOverviewApi(state: FixtureState): ScanApi {
  const api = createFixtureApi();
  if (state === 'empty') return api;
  const complete = state === 'complete';
  const status: Progress['status'] = state;
  const findings =
    state === 'running'
      ? []
      : complete
        ? fixtureFindingSet()
        : [...fixtureFindingSet(), ...issueFindings];
  const progress = progressFor(status, findings);
  api.current = async () => ({
    scope: { id: 'fixture', roots: [fixtureLocation], exclusions: [] },
    progress,
  });
  api.page = async (_generation, offset) => ({
    progress,
    offset,
    nextOffset: null,
    findings,
  });
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
      ] as const
    ).find((candidate) => candidate === requested) ?? 'partial';
  return (
    <ScanProvider api={createOverviewApi(state)}>
      <ScanApplication connectionStatus="SYNTHETIC Overview fixture · browser presentation only" />
    </ScanProvider>
  );
}
