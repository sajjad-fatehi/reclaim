import { invoke } from '@tauri-apps/api/core';
import type { Appearance } from '../shell/appearance';

export interface Exclusion {
  relativePath: string;
  subtree: boolean;
}
export interface Volume {
  id: string;
  measuredAtMs: number;
  totalBytes: string | null;
  freeBytes: string | null;
  availableBytes: string | null;
  usedBytes: string | null;
  metricIssue: string | null;
}
export interface Location {
  id: string;
  displayPath: string;
  volume: Volume;
}
export interface RootPreference {
  key: string | null;
  displayPath: string;
  location: Location | null;
}
export interface Preferences {
  appearance: Appearance;
  roots: RootPreference[];
  exclusions: Exclusion[];
  configured: boolean;
  corrupt: boolean;
}
export interface Scope {
  id: string;
  roots: Location[];
  exclusions: Exclusion[];
}
export type ScanStatus =
  'running' | 'complete' | 'partial' | 'cancelled' | 'failed';
export type CategoryName = 'Documents' | 'Media' | 'Archives' | 'Other files';
export interface CategoryTotal {
  name: CategoryName;
  logicalBytes: string;
  allocatedBytes: string;
  count: number;
}
export interface VolumeAccounting {
  volume: Volume;
  scopeLogicalBytes: string;
  scopeAllocatedBytes: string;
  categories: CategoryTotal[];
  unscannedOrUnexplainedBytes: string | null;
}
export interface Retention {
  nodes: number;
  thresholdBytes: string;
  budgetReached: boolean;
}
export interface Progress {
  generation: number;
  status: ScanStatus;
  startedAtMs: number;
  currentLocation: string | null;
  finishedAtMs: number | null;
  entries: number;
  issueCount: number;
  terminalIssue: string | null;
  logicalBytes: string;
  allocatedBytes: string;
  volumes: VolumeAccounting[];
  retention: Retention;
  sharingUntracked: boolean;
}
export type Detail = 'full' | 'summarized';
export interface Finding {
  id: string;
  parentId: string | null;
  displayName: string;
  kind: 'directory' | 'file' | 'symlink' | 'other' | 'unknown';
  logicalBytes: string;
  allocatedBytes: string;
  measuredAtMs: number;
  modifiedAtMs: number | null;
  identity: string | null;
  aliasOf: string | null;
  complete: boolean;
  issue: string | null;
  foldedItems: number;
  foldedLogicalBytes: string;
  foldedAllocatedBytes: string;
  descendantItems: number;
  detail: Detail;
}
export type ChildSort = 'size' | 'name' | 'modified';
export interface TopFile {
  category: CategoryName;
  displayName: string;
  displayPath: string;
  logicalBytes: string;
  allocatedBytes: string;
  modifiedAtMs: number | null;
  identity: string;
  findingId: string | null;
}
export interface IssueGroup {
  issue: string;
  count: number;
  examples: Finding[];
}
export interface Summary {
  generation: number;
  progress: Progress;
  roots: Finding[];
  topLevel: Finding[];
  topFiles: TopFile[];
  issues: IssueGroup[];
}
export interface FoldedRow {
  items: number;
  logicalBytes: string;
  allocatedBytes: string;
}
export interface Children {
  generation: number;
  parentId: string | null;
  sort: ChildSort;
  total: number;
  nextCursor: string | null;
  findings: Finding[];
  folded: FoldedRow | null;
}
export interface Search {
  generation: number;
  total: number;
  findings: Finding[];
}
/** Temporary shape for the pre-T3 session: bounded summary findings in a single page. */
export interface Page {
  progress: Progress;
  offset: number;
  nextOffset: number | null;
  findings: Finding[];
}
export interface ScanApi {
  loadPreferences: () => Promise<Preferences>;
  savePreferences: (preferences: Preferences) => Promise<void>;
  locations: () => Promise<{
    candidates: Location[];
    unavailableCount: number;
  }>;
  pickFolder: () => Promise<Location | null>;
  register: (locations: string[], exclusions: Exclusion[]) => Promise<Scope>;
  start: (scopeId: string) => Promise<Progress>;
  current: () => Promise<{ scope: Scope | null; progress: Progress | null }>;
  status: (generation: number) => Promise<Progress>;
  summary: (generation: number) => Promise<Summary>;
  children: (
    generation: number,
    parentId: string | null,
    cursor: string | null,
    sort: ChildSort,
  ) => Promise<Children>;
  search: (generation: number, query: string) => Promise<Search>;
  findings: (generation: number, ids: string[]) => Promise<Finding[]>;
  /** Adapter until the session reads the summary directly; removed with the paging loop. */
  page: (generation: number, offset: number) => Promise<Page>;
  cancel: (generation: number) => Promise<Progress>;
}

// Native DTOs own domain validation. These decoders protect presentation from malformed transport.
function record(value: unknown): Record<string, unknown> {
  if (typeof value !== 'object' || value === null || Array.isArray(value))
    throw new Error('Invalid response');
  return Object.fromEntries(Object.entries(value));
}
function string(value: unknown): string {
  if (typeof value !== 'string') throw new Error('Invalid string');
  return value;
}
function number(value: unknown): number {
  if (typeof value !== 'number' || !Number.isSafeInteger(value) || value < 0)
    throw new Error('Invalid number');
  return value;
}
function boolean(value: unknown): boolean {
  if (typeof value !== 'boolean') throw new Error('Invalid boolean');
  return value;
}
function nullable<Value>(
  value: unknown,
  parse: (value: unknown) => Value,
): Value | null {
  return value === null ? null : parse(value);
}
function array<Value>(
  value: unknown,
  parse: (value: unknown) => Value,
): Value[] {
  if (!Array.isArray(value) || value.length > 100_000)
    throw new Error('Invalid array');
  return value.map(parse);
}
function bytes(value: unknown): string {
  const text = string(value);
  if (!/^(0|[1-9]\d{0,19})$/.test(text) || BigInt(text) > 18446744073709551615n)
    throw new Error('Invalid bytes');
  return text;
}
function categoryName(value: unknown): CategoryName {
  if (
    value !== 'Documents' &&
    value !== 'Media' &&
    value !== 'Archives' &&
    value !== 'Other files'
  )
    throw new Error('Invalid category');
  return value;
}
function appearance(value: unknown): Appearance {
  if (value !== 'system' && value !== 'light' && value !== 'dark')
    throw new Error('Invalid appearance');
  return value;
}
function exclusion(value: unknown): Exclusion {
  const data = record(value);
  return {
    relativePath: string(data.relativePath),
    subtree: boolean(data.subtree),
  };
}
function volume(value: unknown): Volume {
  const data = record(value);
  return {
    id: string(data.id),
    measuredAtMs: number(data.measuredAtMs),
    totalBytes: nullable(data.totalBytes, bytes),
    freeBytes: nullable(data.freeBytes, bytes),
    availableBytes: nullable(data.availableBytes, bytes),
    usedBytes: nullable(data.usedBytes, bytes),
    metricIssue: nullable(data.metricIssue, string),
  };
}
function location(value: unknown): Location {
  const data = record(value);
  return {
    id: string(data.id),
    displayPath: string(data.displayPath),
    volume: volume(data.volume),
  };
}
function scope(value: unknown): Scope {
  const data = record(value);
  return {
    id: string(data.id),
    roots: array(data.roots, location),
    exclusions: array(data.exclusions ?? [], exclusion),
  };
}
function progress(value: unknown): Progress {
  const data = record(value);
  const status = data.status;
  if (
    status !== 'running' &&
    status !== 'complete' &&
    status !== 'partial' &&
    status !== 'cancelled' &&
    status !== 'failed'
  )
    throw new Error('Invalid status');
  return {
    generation: number(data.generation),
    status,
    startedAtMs: number(data.startedAtMs),
    currentLocation: nullable(data.currentLocation ?? null, string),
    finishedAtMs: nullable(data.finishedAtMs, number),
    entries: number(data.entries),
    issueCount: number(data.issueCount),
    terminalIssue: nullable(data.terminalIssue, string),
    logicalBytes: bytes(data.logicalBytes),
    allocatedBytes: bytes(data.allocatedBytes),
    volumes: array(data.volumes, (value) => {
      const accounting = record(value);
      return {
        volume: volume(accounting.volume),
        scopeLogicalBytes: bytes(accounting.scopeLogicalBytes),
        scopeAllocatedBytes: bytes(accounting.scopeAllocatedBytes),
        categories: array(accounting.categories, (value) => {
          const category = record(value);
          return {
            name: categoryName(category.name),
            logicalBytes: bytes(category.logicalBytes),
            allocatedBytes: bytes(category.allocatedBytes),
            count: number(category.count),
          };
        }),
        unscannedOrUnexplainedBytes: nullable(
          accounting.unscannedOrUnexplainedBytes,
          bytes,
        ),
      };
    }),
    retention: retention(data.retention),
    sharingUntracked: boolean(data.sharingUntracked),
  };
}
function retention(value: unknown): Retention {
  const data = record(value);
  return {
    nodes: number(data.nodes),
    thresholdBytes: bytes(data.thresholdBytes),
    budgetReached: boolean(data.budgetReached),
  };
}
function finding(value: unknown): Finding {
  const data = record(value);
  const kind = data.kind;
  if (
    kind !== 'directory' &&
    kind !== 'file' &&
    kind !== 'symlink' &&
    kind !== 'other' &&
    kind !== 'unknown'
  )
    throw new Error('Invalid kind');
  return {
    id: string(data.id),
    parentId: nullable(data.parentId, string),
    displayName: string(data.displayName),
    kind,
    logicalBytes: bytes(data.logicalBytes),
    allocatedBytes: bytes(data.allocatedBytes),
    measuredAtMs: number(data.measuredAtMs),
    modifiedAtMs: nullable(data.modifiedAtMs, number),
    identity: nullable(data.identity, string),
    aliasOf: nullable(data.aliasOf, string),
    complete: boolean(data.complete),
    issue: nullable(data.issue, string),
    foldedItems: number(data.foldedItems),
    foldedLogicalBytes: bytes(data.foldedLogicalBytes),
    foldedAllocatedBytes: bytes(data.foldedAllocatedBytes),
    descendantItems: number(data.descendantItems),
    detail: detail(data.detail),
  };
}
function detail(value: unknown): Detail {
  if (value !== 'full' && value !== 'summarized')
    throw new Error('Invalid detail');
  return value;
}
function sort(value: unknown): ChildSort {
  if (value !== 'size' && value !== 'name' && value !== 'modified')
    throw new Error('Invalid sort');
  return value;
}
export function parseSummary(value: unknown): Summary {
  const data = record(value);
  return {
    generation: number(data.generation),
    progress: progress(data.progress),
    roots: array(data.roots, finding),
    topLevel: array(data.topLevel, finding),
    topFiles: array(data.topFiles, (value) => {
      const file = record(value);
      return {
        category: categoryName(file.category),
        displayName: string(file.displayName),
        displayPath: string(file.displayPath),
        logicalBytes: bytes(file.logicalBytes),
        allocatedBytes: bytes(file.allocatedBytes),
        modifiedAtMs: nullable(file.modifiedAtMs, number),
        identity: string(file.identity),
        findingId: nullable(file.findingId, string),
      };
    }),
    issues: array(data.issues, (value) => {
      const group = record(value);
      return {
        issue: string(group.issue),
        count: number(group.count),
        examples: array(group.examples, finding),
      };
    }),
  };
}
export function parseChildren(value: unknown): Children {
  const data = record(value);
  return {
    generation: number(data.generation),
    parentId: nullable(data.parentId, string),
    sort: sort(data.sort),
    total: number(data.total),
    nextCursor: nullable(data.nextCursor, string),
    findings: array(data.findings, finding),
    folded: nullable(data.folded, (value) => {
      const row = record(value);
      return {
        items: number(row.items),
        logicalBytes: bytes(row.logicalBytes),
        allocatedBytes: bytes(row.allocatedBytes),
      };
    }),
  };
}
export function parseSearch(value: unknown): Search {
  const data = record(value);
  return {
    generation: number(data.generation),
    total: number(data.total),
    findings: array(data.findings, finding),
  };
}
async function request<Value>(
  command: string,
  args: Record<string, unknown>,
  parse: (value: unknown) => Value,
): Promise<Value> {
  return parse(await invoke<unknown>(command, args));
}
export const nativeScanApi: ScanApi = {
  loadPreferences: () =>
    request('preferences_load', {}, (value) => {
      const data = record(value);
      return {
        appearance: appearance(data.appearance),
        roots: array(data.roots, (value) => {
          const root = record(value);
          return {
            key: string(root.key),
            displayPath: string(root.displayPath),
            location: nullable(root.location, location),
          };
        }),
        exclusions: array(data.exclusions, exclusion),
        configured: boolean(data.configured),
        corrupt: boolean(data.corrupt),
      };
    }),
  savePreferences: (preferences) =>
    invoke('preferences_save', {
      update: {
        appearance: preferences.appearance,
        roots: preferences.roots.map((root) => ({
          key: root.key,
          locationId: root.location?.id ?? null,
        })),
        exclusions: preferences.exclusions,
      },
    }),
  locations: () =>
    request('scan_locations', {}, (value) => {
      const data = record(value);
      return {
        candidates: array(data.candidates, location),
        unavailableCount: number(data.unavailableCount),
      };
    }),
  pickFolder: () =>
    request('scan_pick_folder', {}, (value) => nullable(value, location)),
  register: (locationIds, exclusions) =>
    request('scan_register_scope', { locationIds, exclusions }, scope),
  start: (scopeId) => request('scan_start', { scopeId }, progress),
  current: () =>
    request('scan_current', {}, (value) => {
      const data = record(value);
      return {
        scope: nullable(data.scope, scope),
        progress: nullable(data.progress, progress),
      };
    }),
  status: (generation) => request('scan_status', { generation }, progress),
  summary: (generation) =>
    request('scan_summary', { generation }, parseSummary),
  children: (generation, parentId, cursor, sort) =>
    request(
      'scan_children',
      { generation, parentId, cursor, limit: 200, sort },
      parseChildren,
    ),
  search: (generation, query) =>
    request('scan_search', { generation, query, limit: 200 }, parseSearch),
  findings: (generation, ids) =>
    request('scan_findings', { generation, ids }, (value) =>
      array(value, finding),
    ),
  page: async (generation) => {
    const current = await nativeScanApi.status(generation);
    if (current.status === 'running')
      return { progress: current, offset: 0, nextOffset: null, findings: [] };
    const summary = await nativeScanApi.summary(generation);
    const unique = new Map<string, Finding>();
    for (const item of [
      ...summary.roots,
      ...summary.topLevel,
      ...summary.issues.flatMap((group) => group.examples),
    ])
      unique.set(item.id, item);
    return {
      progress: summary.progress,
      offset: 0,
      nextOffset: null,
      findings: [...unique.values()],
    };
  },
  cancel: (generation) => request('scan_cancel', { generation }, progress),
};

export function errorCode(error: unknown): string {
  if (
    typeof error === 'object' &&
    error !== null &&
    'code' in error &&
    typeof error.code === 'string'
  )
    return error.code;
  return 'transport';
}
