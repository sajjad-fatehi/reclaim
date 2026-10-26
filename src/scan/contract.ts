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
export interface VolumeAccounting {
  volume: Volume;
  scopeLogicalBytes: string;
  scopeAllocatedBytes: string;
  unscannedOrUnexplainedBytes: string | null;
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
}
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
}
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
        unscannedOrUnexplainedBytes: nullable(
          accounting.unscannedOrUnexplainedBytes,
          bytes,
        ),
      };
    }),
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
  page: (generation, offset) =>
    request('scan_page', { generation, offset, limit: 200 }, (value) => {
      const data = record(value);
      return {
        progress: progress(data.progress),
        offset: number(data.offset),
        nextOffset: nullable(data.nextOffset, number),
        findings: array(data.findings, finding),
      };
    }),
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
