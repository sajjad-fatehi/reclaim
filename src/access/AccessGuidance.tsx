import { ArrowSquareOut } from '@phosphor-icons/react/ArrowSquareOut';
import { LockKey } from '@phosphor-icons/react/LockKey';
import { ShieldCheck } from '@phosphor-icons/react/ShieldCheck';
import { useId } from 'react';
import type { ReactElement } from 'react';
import './access.css';
import { useAccess } from './useAccess';

export type GuidanceContext = 'first-run' | 'settings' | 'denied';

export function OpenSettingsButton(): ReactElement | null {
  const access = useAccess();
  if (!access) return null;
  return (
    <>
      <button className="primary-action" onClick={access.openSettings}>
        Open System Settings
        <ArrowSquareOut size={16} aria-hidden="true" />
      </button>
      {access.openFailed && (
        <p className="access-failure" role="alert">
          Could not open System Settings. Open it yourself, then choose Privacy
          &amp; Security → Full Disk Access.
        </p>
      )}
    </>
  );
}

export function AccessGuidance({
  context,
}: {
  context: GuidanceContext;
}): ReactElement | null {
  const access = useAccess();
  const headingId = useId();
  if (!access?.status) return null;

  if (access.status === 'granted') {
    if (context !== 'settings') return null;
    return (
      <p className="access-granted" role="status">
        <ShieldCheck size={20} weight="fill" aria-hidden="true" />
        Full Disk Access is granted. Reclaim still only measures; it changes
        nothing.
      </p>
    );
  }

  const unknown = access.status === 'unknown';
  return (
    <section className="access-card" aria-labelledby={headingId}>
      <div className="access-icon" aria-hidden="true">
        <LockKey size={24} weight="duotone" />
      </div>
      <div className="access-body">
        <h3 id={headingId}>
          {unknown
            ? 'Full Disk Access status is unknown'
            : 'Allow Full Disk Access for fuller results'}
        </h3>
        <p>
          {unknown
            ? 'Reclaim could not tell whether macOS lets it read protected folders. If a scan reports partial coverage, granting access may help.'
            : context === 'denied'
              ? 'macOS blocked some folders in this scan. Allow Full Disk Access so a new scan can measure them.'
              : 'macOS protects folders such as Mail, Messages, Safari and some app data. Without access, scans skip them and report partial coverage.'}
        </p>
        <p>
          Reclaim only measures sizes, read-only. It never changes, deletes or
          uploads anything. Access is optional: without it the app keeps working
          and reports honest partial coverage.
        </p>
        <ol>
          <li>Choose Open System Settings.</li>
          <li>
            In Privacy &amp; Security → Full Disk Access, turn on Reclaim (use +
            if it is not listed).
          </li>
          <li>
            Come back. Reclaim re-checks when this window is active again. macOS
            may ask you to quit and reopen it.
          </li>
        </ol>
        <div className="access-actions">
          <OpenSettingsButton />
        </div>
        <p className="secondary">
          Development and unsigned builds: macOS grants access to the launching
          app or terminal, and grants may reset after a rebuild until Reclaim is
          signed.
        </p>
      </div>
    </section>
  );
}
