import { invoke } from '@tauri-apps/api/core';

export interface Ownership {
  findingId: string;
  family: string;
  nature: string;
  projectId: string | null;
  confidence: string;
  evidence: { findingId: string; method: string }[];
  consequence: string;
  complete: boolean;
  regenerable: boolean;
}
export interface Inspection {
  findingId: string;
  displayPath: string;
  copyPath: string | null;
  artifact: Ownership | null;
  recognizedAtId: string | null;
}
export interface InspectionApi {
  inspect: (generation: number, findingId: string) => Promise<Inspection>;
  reveal: (generation: number, findingId: string) => Promise<void>;
  copy: (path: string) => Promise<void>;
}
function object(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value))
    throw new Error('Invalid inspection');
  return Object.fromEntries(Object.entries(value));
}
function text(value: unknown): string {
  if (typeof value !== 'string') throw new Error('Invalid inspection text');
  return value;
}
function nullableText(value: unknown): string | null {
  return value === null ? null : text(value);
}
function flag(value: unknown): boolean {
  if (typeof value !== 'boolean') throw new Error('Invalid inspection flag');
  return value;
}
export function parseInspection(value: unknown): Inspection {
  const data = object(value);
  const artifact = data.artifact === null ? null : object(data.artifact);
  if (
    artifact &&
    (!Array.isArray(artifact.evidence) || artifact.evidence.length > 200)
  )
    throw new Error('Invalid evidence');
  return {
    findingId: text(data.findingId),
    displayPath: text(data.displayPath),
    copyPath: nullableText(data.copyPath),
    recognizedAtId: nullableText(data.recognizedAtId),
    artifact: artifact
      ? {
          findingId: text(artifact.findingId),
          family: text(artifact.family),
          nature: text(artifact.nature),
          projectId: nullableText(artifact.projectId),
          confidence: text(artifact.confidence),
          evidence: Array.isArray(artifact.evidence)
            ? artifact.evidence.map((value: unknown) => {
                const evidence = object(value);
                return {
                  findingId: text(evidence.findingId),
                  method: text(evidence.method),
                };
              })
            : [],
          consequence: text(artifact.consequence),
          complete: flag(artifact.complete),
          regenerable: flag(artifact.regenerable),
        }
      : null,
  };
}
export const nativeInspectionApi: InspectionApi = {
  inspect: async (generation, findingId) =>
    parseInspection(
      await invoke<unknown>('scan_inspect', { generation, findingId }),
    ),
  reveal: (generation, findingId) =>
    invoke('scan_reveal', { generation, findingId }),
  copy: (path) => navigator.clipboard.writeText(path),
};
export function actionError(error: unknown): string {
  const data = error && typeof error === 'object' ? object(error) : {};
  if (data.code === 'stale_generation')
    return 'This scan has been replaced. Select a finding from the current scan.';
  if (data.issue === 'vanished')
    return 'Item no longer exists at its measured location. Scan again to refresh.';
  if (data.issue === 'changed')
    return 'Item or an ancestor changed, or is a symbolic link. Scan again; symbolic links are inspection-only.';
  if (data.issue === 'permission_denied' || data.code === 'access_denied')
    return 'Access denied. Check folder permissions and select the location again.';
  return 'Action unavailable. Check access and scan again. Retained measurements remain available.';
}
