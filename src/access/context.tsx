import { useCallback, useEffect, useMemo, useState } from 'react';
import type { ReactElement, ReactNode } from 'react';
import { nativeAccessApi } from './contract';
import type { AccessApi, FullDiskAccess } from './contract';
import { AccessContext } from './useAccess';

export function AccessProvider({
  children,
  api = nativeAccessApi,
}: {
  children: ReactNode;
  api?: AccessApi | undefined;
}): ReactElement {
  const [status, setStatus] = useState<FullDiskAccess | null>(null);
  const [openFailed, setOpenFailed] = useState(false);

  useEffect(() => {
    let active = true;
    const check = (): void => {
      api.status().then(
        (next) => active && setStatus(next),
        () => active && setStatus('unknown'),
      );
    };
    check();
    window.addEventListener('focus', check);
    return () => {
      active = false;
      window.removeEventListener('focus', check);
    };
  }, [api]);

  const openSettings = useCallback((): void => {
    setOpenFailed(false);
    api.openSettings().catch(() => setOpenFailed(true));
  }, [api]);

  const value = useMemo(
    () => ({ status, openFailed, openSettings }),
    [status, openFailed, openSettings],
  );
  return (
    <AccessContext.Provider value={value}>{children}</AccessContext.Provider>
  );
}
