import { getName, getVersion } from '@tauri-apps/api/app';
import { useEffect, useState } from 'react';
import type { ReactElement } from 'react';

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

  return (
    <main>
      <header>
        <p className="eyebrow">Early development</p>
        <h1>Reclaim</h1>
      </header>
      <section aria-labelledby="scaffold-heading">
        <h2 id="scaffold-heading">
          A foundation for understanding disk space.
        </h2>
        <p>
          This is an early development scaffold. Storage scanning and cleanup
          are not available yet.
        </p>
        <p>No files are scanned or changed.</p>
      </section>
      <footer>
        <p role="status">{connectionStatus}</p>
      </footer>
    </main>
  );
}
