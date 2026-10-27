import { ArrowsClockwise } from '@phosphor-icons/react/ArrowsClockwise';
import { CheckCircle } from '@phosphor-icons/react/CheckCircle';
import { Clock } from '@phosphor-icons/react/Clock';
import { Prohibit } from '@phosphor-icons/react/Prohibit';
import { Question } from '@phosphor-icons/react/Question';
import { WarningCircle } from '@phosphor-icons/react/WarningCircle';
import { XCircle } from '@phosphor-icons/react/XCircle';
import type { ReactElement } from 'react';
import type { ItemOutcome } from './contract';

type Outcome = ItemOutcome['outcome'];

const outcomeLabels: Record<Outcome, string> = {
  removed: 'Removed',
  skipped: 'Skipped',
  failed: 'Failed',
  changed: 'Changed since review',
  unknown: 'Unknown · inspect target',
  pending: 'Pending',
};

const outcomeIcons: Record<Outcome, typeof CheckCircle> = {
  removed: CheckCircle,
  skipped: Prohibit,
  failed: XCircle,
  changed: ArrowsClockwise,
  unknown: Question,
  pending: Clock,
};

export function OutcomeBadge({ outcome }: { outcome: Outcome }): ReactElement {
  const Icon = outcomeIcons[outcome];
  return (
    <span className={`outcome-badge outcome-${outcome}`}>
      <Icon aria-hidden="true" size={16} weight="bold" />
      {outcomeLabels[outcome]}
    </span>
  );
}

export function OutcomeSummary({
  items,
}: {
  items: ItemOutcome[];
}): ReactElement | null {
  const counts = new Map<Outcome, number>();
  for (const item of items)
    counts.set(item.outcome, (counts.get(item.outcome) ?? 0) + 1);
  if (counts.size === 0) return null;
  const partial = counts.size > 1 || !counts.has('removed');
  return (
    <p className="outcome-summary">
      {[...counts].map(([outcome, count]) => (
        <span key={outcome} className={`outcome-count outcome-${outcome}`}>
          <strong>{count}</strong> {outcomeLabels[outcome].toLowerCase()}
        </span>
      ))}
      {partial && (
        <span className="secondary">
          Not every item finished as requested. Read each outcome below.
        </span>
      )}
    </p>
  );
}

export function ReversibilityTag({
  method,
}: {
  method: ItemOutcome['method'] | null;
}): ReactElement | null {
  if (method === null) return null;
  const reversible = method === 'trash';
  const Icon = reversible ? CheckCircle : WarningCircle;
  return (
    <span
      className={`reversibility ${reversible ? 'reversible' : 'permanent'}`}
    >
      <Icon aria-hidden="true" size={14} weight="bold" />
      {reversible ? 'Reversible' : 'Not reversible'}
    </span>
  );
}
