import { createContext, useContext } from 'react';
import type { Selection } from './contract';

export interface ReviewSelection {
  selections: Selection[];
  message: string;
  add: (selection: Selection) => void;
  remove: (selection: Selection) => void;
  clear: () => void;
}
export const ReviewContext = createContext<ReviewSelection | null>(null);
export function useReview(): ReviewSelection | null {
  return useContext(ReviewContext);
}
