import type { Artifact, Family, Nature, Worktree } from './contract';
import { familyLabels } from './contract';
import type { Finding } from '../scan/contract';

export type DevelopmentTab = 'projects' | 'tools';
export interface DevelopmentView {
  query: string;
  family: Family | '';
  nature: Nature | '';
  sort: 'size' | 'name';
  descending: boolean;
  page: number;
  selectedId: string | null;
}
export const initialDevelopmentView: DevelopmentView = {
  query: '',
  family: '',
  nature: '',
  sort: 'size',
  descending: true,
  page: 0,
  selectedId: null,
};
export function isProject(artifact: Artifact): boolean {
  return (
    artifact.nature === 'project' ||
    artifact.nature === 'worktree' ||
    artifact.ownedAllocatedBytes !== null
  );
}
export function artifactBytes(
  artifact: Artifact,
  tab: DevelopmentTab,
): string | null {
  return tab === 'projects'
    ? artifact.ownedAllocatedBytes
    : artifact.exclusiveAllocatedBytes;
}
export function worktreeState(worktree: Worktree | null): string {
  if (!worktree) return 'Git state unknown / not recognized';
  return `${worktree.main ? 'Main worktree' : 'Linked worktree'} · ${worktree.detached ? 'detached HEAD' : (worktree.branch ?? 'branch unknown')} · ${worktree.changes} · ${worktree.locked ? 'locked' : worktree.metadataId ? 'no lock observed' : 'lock unknown'}`;
}
export function filterArtifacts(
  artifacts: Artifact[],
  findings: Map<string, Finding>,
  paths: Map<string, string>,
  tab: DevelopmentTab,
  view: DevelopmentView,
): Artifact[] {
  const query = view.query.trim().toLocaleLowerCase();
  return artifacts
    .filter((artifact) => {
      if (tab === 'projects' && !isProject(artifact)) return false;
      if (view.family && artifact.family !== view.family) return false;
      if (view.nature && artifact.nature !== view.nature) return false;
      return (
        !query ||
        [
          findings.get(artifact.findingId)?.displayName,
          paths.get(artifact.findingId),
          familyLabels[artifact.family],
        ].some((value) => value?.toLocaleLowerCase().includes(query))
      );
    })
    .sort((left, right) => {
      let comparison = 0;
      if (view.sort === 'size') {
        const leftBytes = artifactBytes(left, tab);
        const rightBytes = artifactBytes(right, tab);
        if (leftBytes === null || rightBytes === null) {
          if (leftBytes !== rightBytes) return leftBytes === null ? 1 : -1;
        } else
          comparison =
            BigInt(leftBytes) < BigInt(rightBytes)
              ? -1
              : BigInt(leftBytes) > BigInt(rightBytes)
                ? 1
                : 0;
      } else
        comparison = (
          findings.get(left.findingId)?.displayName ?? left.findingId
        ).localeCompare(
          findings.get(right.findingId)?.displayName ?? right.findingId,
        );
      return (
        (view.descending ? -comparison : comparison) ||
        left.findingId.localeCompare(right.findingId)
      );
    });
}
export function relatedWorktrees(
  artifacts: Artifact[],
  project: Artifact,
): Artifact[] {
  const repositoryId = project.worktree?.repositoryId;
  if (!repositoryId) return [];
  return artifacts.filter(
    (artifact) =>
      artifact.findingId !== project.findingId &&
      artifact.worktree?.repositoryId === repositoryId,
  );
}
