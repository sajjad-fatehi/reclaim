import { invoke } from '@tauri-apps/api/core';
import { parseArtifact } from '../development/contract';
import type { Artifact } from '../development/contract';

export type Selection =
  | { source: 'filesystem'; generation: number; findingId: string }
  | { source: 'docker'; findingId: string };
export interface ReviewItem {
  selection: Selection;
  target: string;
  logicalBytes: string;
  allocatedBytes: string | null;
  measuredAtMs: number | null;
  artifact: Artifact | null;
  unavailableReason: string;
  changed: boolean;
  method: ItemOutcome['method'] | null;
}
export interface Review {
  checkedAtMs: number;
  planId: string | null;
  expiresAtMs: number | null;
  items: ReviewItem[];
}
export interface ItemOutcome {
  label: string;
  method: 'trash' | 'git_worktree' | 'docker_image';
  outcome: 'pending' | 'removed' | 'skipped' | 'failed' | 'changed' | 'unknown';
  explanation: string;
}
export interface Operation {
  id: string;
  startedAtMs: number;
  finishedAtMs: number | null;
  items: ItemOutcome[];
  volumeObservations: {
    volumeId: string;
    beforeFreeBytes: string | null;
    afterFreeBytes: string | null;
  }[];
}
export interface HistoryResponse {
  history: { operations: Operation[] } | null;
  unavailable: boolean;
}
export interface Execution {
  planId: string;
  operation: Operation;
  running: boolean;
  persistenceError: boolean;
}
export interface CleanupApi {
  execute: (planId: string) => Promise<Execution>;
  status: () => Promise<Execution | null>;
  cancel: () => Promise<void>;
  review: (selections: Selection[]) => Promise<Review>;
  history: () => Promise<HistoryResponse>;
  clearHistory: () => Promise<void>;
}
export function selectionKey(selection: Selection): string {
  return selection.source === 'filesystem'
    ? `filesystem:${selection.generation}:${selection.findingId}`
    : `docker:${selection.findingId}`;
}
function object(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value))
    throw new Error('Invalid cleanup response');
  return Object.fromEntries(Object.entries(value));
}
function text(value: unknown, maximum = 4096): string {
  if (typeof value !== 'string' || !value || value.length > maximum)
    throw new Error('Invalid cleanup text');
  return value;
}
function integer(value: unknown): number {
  if (typeof value !== 'number' || !Number.isSafeInteger(value) || value < 0)
    throw new Error('Invalid cleanup timestamp');
  return value;
}
function flag(value: unknown): boolean {
  if (typeof value !== 'boolean') throw new Error('Invalid cleanup state');
  return value;
}
function list<Value>(
  value: unknown,
  parse: (entry: unknown) => Value,
  maximum: number,
): Value[] {
  if (!Array.isArray(value) || value.length > maximum)
    throw new Error('Invalid cleanup list');
  return value.map(parse);
}
function choice<const Values extends readonly string[]>(
  value: unknown,
  values: Values,
): Values[number] {
  for (const candidate of values) if (value === candidate) return candidate;
  throw new Error('Invalid cleanup outcome');
}
function bytes(value: unknown): string {
  const result = text(value, 20);
  if (
    !/^(0|[1-9]\d{0,19})$/.test(result) ||
    BigInt(result) > 18446744073709551615n
  )
    throw new Error('Invalid cleanup measurement');
  return result;
}
function selection(value: unknown): Selection {
  const data = object(value);
  const findingId = text(data.findingId, 256);
  if (data.source === 'filesystem')
    return {
      source: 'filesystem',
      generation: integer(data.generation),
      findingId,
    };
  if (data.source === 'docker') return { source: 'docker', findingId };
  throw new Error('Invalid cleanup source');
}
export function parseReview(value: unknown): Review {
  const data = object(value);
  return {
    checkedAtMs: integer(data.checkedAtMs),
    planId: data.planId === null ? null : text(data.planId, 64),
    expiresAtMs: data.expiresAtMs === null ? null : integer(data.expiresAtMs),
    items: list(
      data.items,
      (entry) => {
        const item = object(entry);
        return {
          selection: selection(item.selection),
          target: text(item.target, 16_384),
          logicalBytes:
            item.logicalBytes === 'unknown'
              ? 'unknown'
              : bytes(item.logicalBytes),
          allocatedBytes:
            item.allocatedBytes === null ? null : bytes(item.allocatedBytes),
          measuredAtMs:
            item.measuredAtMs === null ? null : integer(item.measuredAtMs),
          artifact:
            item.artifact === null ? null : parseArtifact(item.artifact),
          unavailableReason: text(item.unavailableReason, 512),
          changed: flag(item.changed),
          method:
            item.method === null
              ? null
              : choice(item.method, ['trash', 'git_worktree', 'docker_image']),
        };
      },
      100,
    ),
  };
}
export function parseHistory(value: unknown): HistoryResponse {
  const data = object(value);
  const unavailable = flag(data.unavailable);
  if (unavailable) {
    if (data.history !== null) throw new Error('Invalid history state');
    return { history: null, unavailable };
  }
  const history = object(data.history);
  return {
    unavailable,
    history: {
      operations: list(
        history.operations,
        (entry) => {
          const operation = object(entry);
          return {
            id: text(operation.id, 64),
            startedAtMs: integer(operation.startedAtMs),
            finishedAtMs:
              operation.finishedAtMs === null
                ? null
                : integer(operation.finishedAtMs),
            volumeObservations:
              operation.volumeObservations === undefined
                ? []
                : list(
                    operation.volumeObservations,
                    (entry) => {
                      const volume = object(entry);
                      return {
                        volumeId: text(volume.volumeId, 128),
                        beforeFreeBytes:
                          volume.beforeFreeBytes === null
                            ? null
                            : bytes(volume.beforeFreeBytes),
                        afterFreeBytes:
                          volume.afterFreeBytes === null
                            ? null
                            : bytes(volume.afterFreeBytes),
                      };
                    },
                    16,
                  ),
            items: list(
              operation.items,
              (entry) => {
                const item = object(entry);
                return {
                  label: text(item.label),
                  method: choice(item.method, [
                    'trash',
                    'git_worktree',
                    'docker_image',
                  ]),
                  outcome: choice(item.outcome, [
                    'pending',
                    'removed',
                    'skipped',
                    'failed',
                    'changed',
                    'unknown',
                  ]),
                  explanation: text(item.explanation, 512),
                };
              },
              100,
            ),
          };
        },
        50,
      ),
    },
  };
}
export function parseExecution(value: unknown): Execution {
  const data = object(value);
  const history = parseHistory({
    unavailable: false,
    history: { operations: [data.operation] },
  });
  const operation = history.history?.operations[0];
  if (!operation) throw new Error('Missing operation');
  return {
    planId: text(data.planId, 64),
    operation,
    running: flag(data.running),
    persistenceError: flag(data.persistenceError),
  };
}
export const nativeCleanupApi: CleanupApi = {
  execute: async (planId) =>
    parseExecution(
      await invoke<unknown>('cleanup_execute', { planId, confirmed: true }),
    ),
  status: async () => {
    const result = await invoke<unknown>('cleanup_status');
    return result === null ? null : parseExecution(result);
  },
  cancel: () => invoke('cleanup_cancel'),
  review: async (selections) =>
    parseReview(await invoke<unknown>('cleanup_review', { selections })),
  history: async () => parseHistory(await invoke<unknown>('cleanup_history')),
  clearHistory: () => invoke('cleanup_clear_history'),
};
