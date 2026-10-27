import type { ReactElement } from 'react';
import { selectionKey } from './contract';
import type { Selection } from './contract';
import { useReview } from './useReview';

export function ReviewButton({
  selection,
  disabled = false,
}: {
  selection: Selection;
  disabled?: boolean;
}): ReactElement | null {
  const review = useReview();
  if (!review) return null;
  const selected = review.selections.some(
    (entry) => selectionKey(entry) === selectionKey(selection),
  );
  return (
    <button
      disabled={disabled || selected || review.selections.length >= 100}
      onClick={() => review.add(selection)}
    >
      {selected ? 'Queued for eligibility review' : 'Review cleanup options'}
    </button>
  );
}
