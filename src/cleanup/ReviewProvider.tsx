import { useState } from 'react';
import type { ReactElement, ReactNode } from 'react';
import { selectionKey } from './contract';
import type { Selection } from './contract';
import { ReviewContext } from './useReview';

export function ReviewProvider({
  children,
}: {
  children: ReactNode;
}): ReactElement {
  const [selections, setSelections] = useState<Selection[]>([]);
  const [message, setMessage] = useState('');
  return (
    <ReviewContext.Provider
      value={{
        selections,
        message,
        add: (selection) => {
          setSelections((current) => {
            if (
              current.some(
                (entry) => selectionKey(entry) === selectionKey(selection),
              )
            )
              return current;
            if (current.length >= 100) return current;
            return [...current, selection];
          });
          setMessage(
            'Item queued for eligibility review. Selection alone grants no removal authority. Maximum 100 items.',
          );
        },
        remove: (selection) => {
          setSelections((current) =>
            current.filter(
              (entry) => selectionKey(entry) !== selectionKey(selection),
            ),
          );
          setMessage('Item removed from review selection. Files unchanged.');
        },
        clear: () => {
          setSelections([]);
          setMessage('Review selection cleared. Files unchanged.');
        },
      }}
    >
      {children}
    </ReviewContext.Provider>
  );
}
