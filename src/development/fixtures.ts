import type { Finding } from '../scan/contract';
import {
  createFixtureApi,
  fixtureFindings,
  fixtureLocation,
  fixtureProgress,
  serveScan,
} from '../scan/fixtures';
import type { InspectionApi } from '../explore/contract';
import { createTreeApi, pathOf } from '../explore/treeApi';
import type {
  Artifact,
  DevelopmentApi,
  DockerItem,
  DockerSnapshot,
  Family,
  Nature,
  RecognitionPage,
} from './contract';
import { familyLabels } from './contract';

export function fixtureArtifact(
  findingId: string,
  family: Family = 'java_script',
  nature: Nature = 'project',
): Artifact {
  return {
    findingId,
    family,
    nature,
    projectId: nature === 'project' ? findingId : null,
    confidence: 'verified',
    evidence: [{ findingId, method: 'SYNTHETIC verified fixture metadata' }],
    consequence:
      'SYNTHETIC fixture: source, secrets and persistent data must be preserved. Inspection only.',
    cleanupSupport: 'inspection_only',
    complete: true,
    regenerable: false,
    exclusiveLogicalBytes: '1024',
    exclusiveAllocatedBytes: '4096',
    ownedLogicalBytes: nature === 'project' ? '3072' : null,
    ownedAllocatedBytes: nature === 'project' ? '12288' : null,
    worktree: null,
  };
}
export function developmentFixture(): {
  findings: Finding[];
  report: RecognitionPage;
  api: DevelopmentApi;
  inspectionApi: InspectionApi;
  scanApi: ReturnType<typeof createFixtureApi>;
} {
  const root = fixtureFindings()[0];
  if (!root) throw new Error('Missing fixture root');
  const project = fixtureArtifact('1:1');
  project.worktree = {
    repositoryId: '1:1',
    metadataId: '1:90',
    branch: 'main',
    main: true,
    detached: false,
    locked: false,
    changes: 'dirty',
  };
  const dependencies = {
    ...fixtureArtifact('1:2', 'java_script', 'dependencies'),
    projectId: project.findingId,
  };
  const build = {
    ...fixtureArtifact('1:3', 'rust', 'build'),
    projectId: project.findingId,
  };
  const logs = {
    ...fixtureArtifact('1:4', 'project_logs', 'logs'),
    projectId: project.findingId,
  };
  const worktree = fixtureArtifact('1:5', 'codex', 'worktree');
  worktree.ownedAllocatedBytes = '4096';
  worktree.ownedLogicalBytes = '1024';
  worktree.projectId = worktree.findingId;
  worktree.worktree = {
    repositoryId: project.findingId,
    metadataId: '1:91',
    branch: 'feat/SYNTHETIC-開発',
    main: false,
    detached: false,
    locked: true,
    changes: 'unknown',
  };
  const families = Object.keys(familyLabels).filter(
    (family): family is Family => family in familyLabels,
  );
  const artifacts = [
    project,
    dependencies,
    build,
    logs,
    worktree,
    ...families.map((family, index) =>
      fixtureArtifact(
        `1:${index + 10}`,
        family,
        ['nvm', 'go', 'homebrew'].includes(family)
          ? 'runtime'
          : ['vs_code', 'cursor', 'jet_brains', 'claude', 'codex'].includes(
                family,
              )
            ? 'editor_state'
            : family === 'protected'
              ? 'protected'
              : family === 'temporary'
                ? 'temporary'
                : 'shared_store',
      ),
    ),
  ];
  const names = new Map([
    ['1:1', 'SYNTHETIC Atlas – 開発'],
    ['1:2', 'node_modules'],
    ['1:3', 'target'],
    ['1:4', 'logs'],
    ['1:5', 'SYNTHETIC linked worktree'],
  ]);
  const findings: Finding[] = [
    root,
    ...artifacts.map((artifact) => ({
      ...root,
      id: artifact.findingId,
      parentId:
        artifact.projectId && artifact.projectId !== artifact.findingId
          ? artifact.projectId
          : root.id,
      displayName:
        names.get(artifact.findingId) ??
        `SYNTHETIC ${familyLabels[artifact.family]}`,
      identity: `fixture:${artifact.findingId}`,
    })),
  ];
  const report: RecognitionPage = {
    generation: 1,
    offset: 0,
    nextOffset: null,
    finished: true,
    limited: false,
    cancelled: false,
    diagnostics: [
      { findingId: '1:5', reason: 'SYNTHETIC_locked_worktree_changes_unknown' },
    ],
    diagnosticsTruncated: false,
    findings: artifacts,
  };
  const api: DevelopmentApi = {
    recognitionPage: async () => report,
    dockerInventory: async () => dockerFixture(),
    cancelDocker: async () => undefined,
  };
  const inspectionApi: InspectionApi = {
    inspect: async (_generation, findingId) => ({
      findingId,
      displayPath: pathOf(findings, findingId),
      copyPath: pathOf(findings, findingId),
      artifact:
        artifacts.find((artifact) => artifact.findingId === findingId) ?? null,
      recognizedAtId: findingId,
    }),
    reveal: async () => {
      throw { code: 'unavailable' };
    },
    copy: async () => undefined,
  };
  const scanApi = createTreeApi(createFixtureApi(), findings);
  scanApi.current = async () => ({
    scope: { id: 'SYNTHETIC', roots: [fixtureLocation], exclusions: [] },
    progress: fixtureProgress('partial'),
  });
  serveScan(scanApi, fixtureProgress('partial'), findings);
  return { findings, report, api, inspectionApi, scanApi };
}
export function dockerFixture(): DockerSnapshot {
  const items: DockerItem[] = (
    ['image', 'container', 'build_cache', 'volume'] as const
  ).map((kind, index) => ({
    artifact: fixtureArtifact(
      `docker:${index}`,
      'docker',
      kind === 'volume' ? 'protected' : 'build',
    ),
    kind,
    identity: `SYNTHETIC-${kind}-identity`,
    references: kind === 'image' ? 2 : null,
    active: kind === 'container' ? true : kind === 'volume' ? null : false,
    persistent: kind === 'volume' || kind === 'container',
    shared: kind === 'image' ? true : null,
    logicalBytes: kind === 'volume' ? null : '8192',
    sharedBytes: kind === 'image' ? '4096' : null,
    metricBasis: 'SYNTHETIC Docker logical observation; not physical recovery',
  }));
  return {
    measuredAtMs: '2000',
    result: {
      Ok: {
        items,
        accounting:
          'SYNTHETIC inventory. Docker observations do not add to filesystem totals.',
      },
    },
  };
}
