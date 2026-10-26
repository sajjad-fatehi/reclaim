import { getName, getVersion } from '@tauri-apps/api/app';
import { useEffect, useState } from 'react';
import type { ReactElement } from 'react';
import { AppShell } from './shell/AppShell';

export function App(): ReactElement {
  const [connectionStatus, setConnectionStatus] = useState(
    'Connecting to the desktop app…',
  );

  useEffect(() => {
    let active = true;

    void Promise.all([getName(), getVersion()]).then(
      ([name, version]) => {
        if (active) {
          setConnectionStatus(`${name} ${version} · Desktop connected`);
        }
      },
      () => {
        if (active) {
          setConnectionStatus(
            'Desktop connection unavailable. Close and reopen Reclaim to try again.',
          );
        }
      },
    );

    return () => {
      active = false;
    };
  }, []);

  return <AppShell connectionStatus={connectionStatus} />;
}
