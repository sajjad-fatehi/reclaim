import { useEffect, useState } from 'react';
import type { ReactElement, ReactNode } from 'react';
import { nativeScanApi } from './contract';
import type { ScanApi } from './contract';
import { ScanSession } from './session';
import { ScanContext } from './useScanSession';

export function ScanProvider({
  children,
  api = nativeScanApi,
}: {
  children: ReactNode;
  api?: ScanApi;
}): ReactElement {
  const [session] = useState(() => new ScanSession(api));
  useEffect(() => session.connect(), [session]);
  return (
    <ScanContext.Provider value={session}>{children}</ScanContext.Provider>
  );
}
