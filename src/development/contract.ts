import { invoke } from '@tauri-apps/api/core';
import { errorCode } from '../scan/contract';
import type { Ownership } from '../explore/contract';

export const familyLabels = {
  docker: 'Docker',
  java_script: 'JavaScript',
  python: 'Python',
  rust: 'Rust',
  project_logs: 'Project logs',
  npm: 'npm',
  pnpm: 'pnpm',
  yarn: 'Yarn',
  bun: 'Bun',
  pip: 'pip',
  uv: 'uv',
  nvm: 'nvm',
  go: 'Go',
  cargo: 'Cargo',
  homebrew: 'Homebrew',
  vs_code: 'VS Code',
  cursor: 'Cursor',
  jet_brains: 'JetBrains',
  claude: 'Claude',
  codex: 'Codex',
  git: 'Git',
  temporary: 'Temporary',
  protected: 'Protected',
} satisfies Record<string, string>;
export type Family = keyof typeof familyLabels;
export const natureLabels = {
  project: 'Source / configuration',
  dependencies: 'Dependencies',
  build: 'Build outputs',
  logs: 'Logs',
  shared_store: 'Shared stores',
  runtime: 'Runtimes',
  editor_state: 'Editor / coding-tool state',
  worktree: 'Worktrees',
  temporary: 'Temporary',
  protected: 'Protected data',
} satisfies Record<string, string>;
export type Nature = keyof typeof natureLabels;
export interface Worktree {
  repositoryId: string | null;
  metadataId: string | null;
  branch: string | null;
  main: boolean;
  detached: boolean;
  locked: boolean;
  changes: 'clean' | 'dirty' | 'untracked' | 'ignored' | 'unknown';
}
export interface Artifact extends Ownership {
  family: Family;
  nature: Nature;
  confidence: 'verified' | 'contextual';
  cleanupSupport: 'inspection_only';
  exclusiveLogicalBytes: string;
  exclusiveAllocatedBytes: string;
  ownedLogicalBytes: string | null;
  ownedAllocatedBytes: string | null;
  worktree: Worktree | null;
}
export interface RecognitionPage {
  generation: number;
  offset: number;
  nextOffset: number | null;
  finished: boolean;
  limited: boolean;
  cancelled: boolean;
  findings: Artifact[];
  diagnostics: { findingId: string; reason: string }[];
  diagnosticsTruncated: boolean;
}
export interface DockerItem {
  artifact: Artifact;
  kind: 'image' | 'container' | 'build_cache' | 'volume';
  identity: string;
  references: number | null;
  active: boolean | null;
  persistent: boolean;
  shared: boolean | null;
  logicalBytes: string | null;
  sharedBytes: string | null;
  metricBasis: string;
}
export const dockerFailures = {
  unavailable:
    'Docker CLI unavailable. Install or configure it outside Reclaim, then retry.',
  unsupported_context:
    'Only a supported local Unix-socket Docker context can be inspected. Remote contexts are not contacted.',
  unsupported_version:
    'Docker version unsupported. Inventory requires Docker 24–29 and API 1.43–1.52.',
  unsupported_engine:
    'Engine unsupported. Only Docker Engine or Docker Desktop inventory is supported.',
  daemon_unavailable:
    'Local Docker daemon unavailable. Reclaim does not start it. Retry after it is available.',
  malformed: 'Docker returned invalid inventory. Retry after checking Docker.',
  limited:
    'Docker inventory exceeded its output limit. No complete inventory available.',
  timeout: 'Docker inventory timed out. Retry when the local engine responds.',
  cancelled: 'Docker inventory cancelled. Retry to measure again.',
  busy: 'Docker inventory already running. Wait for it to finish, then retry.',
  access_denied:
    'Docker access denied. Check local socket permissions; Reclaim never bypasses access.',
} satisfies Record<string, string>;
export function dockerErrorMessage(error: unknown): string {
  for (const [failure, message] of Object.entries(dockerFailures)) {
    if (error === failure) return message;
  }
  return 'Docker connection unavailable. Check desktop access and retry.';
}
export interface DockerSnapshot {
  measuredAtMs: string;
  result:
    | { Ok: { items: DockerItem[]; accounting: string } }
    | { Err: keyof typeof dockerFailures };
}
export interface DevelopmentApi {
  recognitionPage: (
    generation: number,
    offset: number,
  ) => Promise<RecognitionPage>;
  dockerInventory: () => Promise<DockerSnapshot>;
  cancelDocker: () => Promise<void>;
}

function object(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value))
    throw new Error('Invalid development response');
  return Object.fromEntries(Object.entries(value));
}
function text(value: unknown): string {
  if (typeof value !== 'string') throw new Error('Invalid text');
  return value;
}
function flag(value: unknown): boolean {
  if (typeof value !== 'boolean') throw new Error('Invalid flag');
  return value;
}
function integer(value: unknown): number {
  if (typeof value !== 'number' || !Number.isSafeInteger(value) || value < 0)
    throw new Error('Invalid integer');
  return value;
}
function bytes(value: unknown): string {
  const result = text(value);
  if (
    !/^(0|[1-9]\d{0,19})$/.test(result) ||
    BigInt(result) > 18446744073709551615n
  )
    throw new Error('Invalid byte count');
  return result;
}
function nullable<Value>(
  value: unknown,
  parse: (value: unknown) => Value,
): Value | null {
  return value === null ? null : parse(value);
}
function list<Value>(
  value: unknown,
  parse: (value: unknown) => Value,
): Value[] {
  if (!Array.isArray(value) || value.length > 100_000)
    throw new Error('Invalid list');
  return value.map(parse);
}
function choice<const Values extends readonly string[]>(
  value: unknown,
  values: Values,
): Values[number] {
  for (const candidate of values) if (value === candidate) return candidate;
  throw new Error('Invalid state');
}
function key<Keys extends string>(
  value: unknown,
  labels: Record<Keys, string>,
): Keys {
  for (const candidate in labels) if (value === candidate) return candidate;
  throw new Error('Invalid category');
}
export function parseArtifact(value: unknown): Artifact {
  const data = object(value);
  const worktree = data.worktree === null ? null : object(data.worktree);
  return {
    findingId: text(data.findingId),
    family: key(data.family, familyLabels),
    nature: key(data.nature, natureLabels),
    projectId: nullable(data.projectId, text),
    confidence: choice(data.confidence, ['verified', 'contextual']),
    evidence: list(data.evidence, (value) => {
      const evidence = object(value);
      return {
        findingId: text(evidence.findingId),
        method: text(evidence.method),
      };
    }),
    consequence: text(data.consequence),
    complete: flag(data.complete),
    regenerable: flag(data.regenerable),
    cleanupSupport: choice(data.cleanupSupport, ['inspection_only']),
    exclusiveLogicalBytes: bytes(data.exclusiveLogicalBytes),
    exclusiveAllocatedBytes: bytes(data.exclusiveAllocatedBytes),
    ownedLogicalBytes: nullable(data.ownedLogicalBytes, bytes),
    ownedAllocatedBytes: nullable(data.ownedAllocatedBytes, bytes),
    worktree: worktree
      ? {
          repositoryId: nullable(worktree.repositoryId, text),
          metadataId: nullable(worktree.metadataId, text),
          branch: nullable(worktree.branch, text),
          main: flag(worktree.main),
          detached: flag(worktree.detached),
          locked: flag(worktree.locked),
          changes: choice(worktree.changes, [
            'clean',
            'dirty',
            'untracked',
            'ignored',
            'unknown',
          ]),
        }
      : null,
  };
}
export function parseRecognitionPage(value: unknown): RecognitionPage {
  const data = object(value);
  return {
    generation: integer(data.generation),
    offset: integer(data.offset),
    nextOffset: nullable(data.nextOffset, integer),
    finished: flag(data.finished),
    limited: flag(data.limited),
    cancelled: flag(data.cancelled),
    findings: list(data.findings, parseArtifact),
    diagnosticsTruncated: flag(data.diagnosticsTruncated),
    diagnostics: list(data.diagnostics, (value) => {
      const diagnostic = object(value);
      return {
        findingId: text(diagnostic.findingId),
        reason: text(diagnostic.reason),
      };
    }),
  };
}
export function parseDockerSnapshot(value: unknown): DockerSnapshot {
  const data = object(value);
  const result = object(data.result);
  if ('Err' in result)
    return {
      measuredAtMs: bytes(data.measuredAtMs),
      result: { Err: key(result.Err, dockerFailures) },
    };
  const inventory = object(result.Ok);
  return {
    measuredAtMs: bytes(data.measuredAtMs),
    result: {
      Ok: {
        accounting: text(inventory.accounting),
        items: list(inventory.items, (value) => {
          const item = object(value);
          return {
            artifact: parseArtifact(item.artifact),
            kind: choice(item.kind, [
              'image',
              'container',
              'build_cache',
              'volume',
            ]),
            identity: text(item.identity),
            references: nullable(item.references, integer),
            active: nullable(item.active, flag),
            persistent: flag(item.persistent),
            shared: nullable(item.shared, flag),
            logicalBytes: nullable(item.logicalBytes, bytes),
            sharedBytes: nullable(item.sharedBytes, bytes),
            metricBasis: text(item.metricBasis),
          };
        }),
      },
    },
  };
}
export async function loadRecognition(
  api: DevelopmentApi,
  generation: number,
): Promise<RecognitionPage> {
  let offset = 0;
  let report: RecognitionPage | null = null;
  const identities = new Set<string>();
  for (;;) {
    let page: RecognitionPage;
    try {
      page = await api.recognitionPage(generation, offset);
    } catch (error) {
      if (errorCode(error) !== 'rate_limited') throw error;
      await new Promise<void>((resolve) => setTimeout(resolve, 120));
      page = await api.recognitionPage(generation, offset);
    }
    if (
      page.generation !== generation ||
      page.offset !== offset ||
      page.findings.length > 200 ||
      (page.nextOffset !== null &&
        page.nextOffset !== offset + page.findings.length)
    )
      throw new Error('Invalid recognition page');
    for (const artifact of page.findings) {
      if (
        !artifact.findingId.startsWith(`${generation}:`) ||
        identities.has(artifact.findingId)
      )
        throw new Error('Invalid recognition identity');
      identities.add(artifact.findingId);
    }
    report = report
      ? {
          ...page,
          findings: [...report.findings, ...page.findings],
          diagnostics: report.diagnostics,
        }
      : page;
    if (page.nextOffset === null) return report;
    if (page.nextOffset <= offset || page.nextOffset >= 100_000)
      throw new Error('Invalid recognition continuation');
    offset = page.nextOffset;
  }
}
export const nativeDevelopmentApi: DevelopmentApi = {
  recognitionPage: async (generation, offset) =>
    parseRecognitionPage(
      await invoke<unknown>('recognition_page', {
        generation,
        offset,
        limit: 200,
      }),
    ),
  dockerInventory: async () =>
    parseDockerSnapshot(await invoke<unknown>('docker_inventory')),
  cancelDocker: () => invoke('docker_cancel'),
};
