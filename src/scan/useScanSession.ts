import { createContext, useContext, useSyncExternalStore } from 'react';
import { ScanSession } from './session';
import type { ScanState } from './session';

export const ScanContext = createContext<ScanSession | null>(null);
export function useScanSession(): { state: ScanState; session: ScanSession } {
  const session = useContext(ScanContext);
  if (!session) throw new Error('ScanProvider is required');
  const state = useSyncExternalStore(session.subscribe, session.getSnapshot);
  return { state, session };
}
