import { createContext, useContext } from 'react';
import type { FullDiskAccess } from './contract';

export interface AccessState {
  status: FullDiskAccess | null;
  openFailed: boolean;
  openSettings: () => void;
}

export const AccessContext = createContext<AccessState | null>(null);

// Null outside a provider keeps screens usable where guidance is not wired.
export function useAccess(): AccessState | null {
  return useContext(AccessContext);
}
