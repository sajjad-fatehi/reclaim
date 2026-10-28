import { Cube } from '@phosphor-icons/react/Cube';
import { Database } from '@phosphor-icons/react/Database';
import { Folder } from '@phosphor-icons/react/Folder';
import { GitBranch } from '@phosphor-icons/react/GitBranch';
import { Hammer } from '@phosphor-icons/react/Hammer';
import { Lock } from '@phosphor-icons/react/Lock';
import { Package } from '@phosphor-icons/react/Package';
import { PencilSimpleLine } from '@phosphor-icons/react/PencilSimpleLine';
import { Scroll } from '@phosphor-icons/react/Scroll';
import { ShieldCheck } from '@phosphor-icons/react/ShieldCheck';
import { Stack } from '@phosphor-icons/react/Stack';
import { Warning } from '@phosphor-icons/react/Warning';
import type { ComponentType, ReactElement } from 'react';
import type { Artifact, Nature } from './contract';

type IconComponent = ComponentType<{ size?: number; 'aria-hidden'?: 'true' }>;
type Category =
  | 'dependencies'
  | 'build'
  | 'docker'
  | 'stores'
  | 'editor'
  | 'worktrees'
  | 'logs'
  | 'unknown';

interface Presentation {
  icon: IconComponent;
  category: Category;
}

const natureIcons: Record<Nature, Presentation> = {
  project: { icon: Folder, category: 'unknown' },
  dependencies: { icon: Package, category: 'dependencies' },
  build: { icon: Hammer, category: 'build' },
  logs: { icon: Scroll, category: 'logs' },
  shared_store: { icon: Database, category: 'stores' },
  runtime: { icon: Stack, category: 'stores' },
  editor_state: { icon: PencilSimpleLine, category: 'editor' },
  worktree: { icon: GitBranch, category: 'worktrees' },
  temporary: { icon: Scroll, category: 'logs' },
  protected: { icon: ShieldCheck, category: 'unknown' },
};

function presentationFor(
  artifact: Pick<Artifact, 'family' | 'nature'>,
): Presentation {
  if (artifact.family === 'docker') return { icon: Cube, category: 'docker' };
  return natureIcons[artifact.nature];
}

export function ArtifactChip({
  artifact,
}: {
  artifact: Pick<Artifact, 'family' | 'nature'>;
}): ReactElement {
  const { icon: Icon, category } = presentationFor(artifact);
  return (
    <span className="artifact-chip" data-category={category}>
      <Icon size={20} aria-hidden="true" />
    </span>
  );
}

export function DockerChip(): ReactElement {
  return (
    <span className="artifact-chip" data-category="docker">
      <Cube size={20} aria-hidden="true" />
    </span>
  );
}

export function CautionBadge({
  kind,
  children,
}: {
  kind: 'worktree' | 'persistent';
  children: string;
}): ReactElement {
  const Icon = kind === 'persistent' ? Lock : Warning;
  return (
    <span className="caution-badge" data-kind={kind}>
      <Icon size={16} aria-hidden="true" />
      {children}
    </span>
  );
}
