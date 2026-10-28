import { invoke } from '@tauri-apps/api/core';

export type FullDiskAccess = 'granted' | 'not_granted' | 'unknown';

export interface AccessApi {
  status: () => Promise<FullDiskAccess>;
  openSettings: () => Promise<void>;
}

const statuses: readonly FullDiskAccess[] = [
  'granted',
  'not_granted',
  'unknown',
];

export function decodeAccessStatus(value: unknown): FullDiskAccess {
  const status =
    typeof value === 'object' && value !== null && 'fullDiskAccess' in value
      ? value.fullDiskAccess
      : null;
  return statuses.find((known) => known === status) ?? 'unknown';
}

export const nativeAccessApi: AccessApi = {
  status: async () => decodeAccessStatus(await invoke('access_status')),
  openSettings: () => invoke('access_open_settings'),
};
