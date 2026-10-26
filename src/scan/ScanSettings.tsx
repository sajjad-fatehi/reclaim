import { useId, useState } from 'react';
import type { ReactElement } from 'react';
import { FolderPlus } from '@phosphor-icons/react/FolderPlus';
import { Notice, Toolbar } from '../shell/primitives';
import { useScanSession } from './useScanSession';
import { validExclusion } from './session';

export function ScanSettings(): ReactElement {
  const { state, session } = useScanSession();
  const [relativePath, setRelativePath] = useState('');
  const [subtree, setSubtree] = useState(true);
  const [validation, setValidation] = useState<string | null>(null);
  const inputId = useId();
  const preferences = state.preferences;
  const selectedIds = new Set(
    preferences.roots.flatMap((root) =>
      root.location ? [root.location.id] : [],
    ),
  );
  const suggestions = state.candidates.filter(
    (candidate) =>
      !selectedIds.has(candidate.id) &&
      !preferences.roots.some(
        (root) => root.displayPath === candidate.displayPath,
      ),
  );
  const running = state.snapshot?.progress.status === 'running';

  return (
    <section
      className="scan-settings"
      aria-label="Scan locations and exclusions"
    >
      <h2>Scan locations</h2>
      <p>
        Find what is using your disk space. Review locations before starting;
        Read-only scanning measures filesystem metadata and selectively reads
        project/tool metadata for recognition. No files are changed.
      </p>
      {state.loading ? (
        <p>Loading saved settings and suggested locations…</p>
      ) : (
        <>
          {preferences.corrupt && (
            <Notice>
              Saved settings are corrupt or unreadable. Safe defaults loaded;
              original file stays untouched until you save.
            </Notice>
          )}
          {preferences.roots.length === 0 && (
            <Notice>
              No locations selected. Add a readable folder to begin.
            </Notice>
          )}
          <ul className="scope-list">
            {preferences.roots.map((root, index) => (
              <li key={root.location?.id ?? root.key}>
                <div>
                  <strong className="path-text">{root.displayPath}</strong>
                  <span className="secondary">
                    {root.location
                      ? 'Available at selection · access checked again on start'
                      : 'Unavailable · volume missing, moved folder or access required'}
                  </span>
                </div>
                <button
                  type="button"
                  onClick={() =>
                    session.editPreferences({
                      roots: preferences.roots.filter(
                        (_, rootIndex) => rootIndex !== index,
                      ),
                    })
                  }
                  aria-label={`Remove ${root.displayPath}`}
                >
                  Remove
                </button>
              </li>
            ))}
          </ul>
          <Toolbar label="Manage scan locations">
            <button
              type="button"
              onClick={() => void session.pickFolder()}
              disabled={state.busy || preferences.roots.length >= 16}
            >
              <FolderPlus size={18} aria-hidden="true" /> Add folder…
            </button>
            {suggestions.length > 0 && (
              <label>
                Add suggestion{' '}
                <select
                  aria-label="Add suggested location"
                  value=""
                  disabled={preferences.roots.length >= 16}
                  onChange={(event) => {
                    const location = suggestions.find(
                      (candidate) => candidate.id === event.target.value,
                    );
                    if (location)
                      session.editPreferences({
                        roots: [
                          ...preferences.roots,
                          {
                            key: null,
                            displayPath: location.displayPath,
                            location,
                          },
                        ],
                      });
                  }}
                >
                  <option value="">Choose location…</option>
                  {suggestions.map((candidate) => (
                    <option value={candidate.id} key={candidate.id}>
                      {candidate.displayPath}
                    </option>
                  ))}
                </select>
              </label>
            )}
          </Toolbar>
          {state.unavailableCount > 0 && (
            <p>
              {state.unavailableCount} suggested locations unavailable. Add a
              folder or review macOS Privacy &amp; Security access.
            </p>
          )}
          <p className="secondary">
            Nested locations on the same mount count once. Other mounts require
            their own selection. No permission bypass or automatic scan on
            launch.
          </p>
          <h2>Exclusions</h2>
          <p id={`${inputId}-help`}>
            Literal, case-sensitive paths relative to each selected root.
            Subtree skips that entry and descendants; Exact file skips only that
            file, leaving directories traversable. No wildcards.
          </p>
          <ul className="exclusion-list">
            {preferences.exclusions.map((exclusion, index) => (
              <li key={`${exclusion.relativePath}:${exclusion.subtree}`}>
                <span className="path-text">
                  {exclusion.relativePath} ·{' '}
                  {exclusion.subtree ? 'Subtree' : 'Exact file'}
                </span>
                <button
                  type="button"
                  aria-label={`Remove exclusion ${exclusion.relativePath}`}
                  onClick={() =>
                    session.editPreferences({
                      exclusions: preferences.exclusions.filter(
                        (_, ruleIndex) => ruleIndex !== index,
                      ),
                    })
                  }
                >
                  Remove
                </button>
              </li>
            ))}
          </ul>
          <form
            className="exclusion-form"
            onSubmit={(event) => {
              event.preventDefault();
              if (!validExclusion(relativePath)) {
                setValidation(
                  'Enter a relative path without empty, dot or parent components.',
                );
                return;
              }
              if (
                preferences.exclusions.some(
                  (rule) =>
                    rule.relativePath === relativePath &&
                    rule.subtree === subtree,
                )
              ) {
                setValidation('This exclusion already exists.');
                return;
              }
              session.editPreferences({
                exclusions: [
                  ...preferences.exclusions,
                  { relativePath, subtree },
                ],
              });
              setRelativePath('');
              setValidation(null);
            }}
          >
            <label htmlFor={inputId}>
              Relative path
              <input
                id={inputId}
                value={relativePath}
                onChange={(event) => setRelativePath(event.target.value)}
                placeholder="Library/Caches"
                maxLength={1024}
                aria-describedby={`${inputId}-help`}
              />
            </label>
            <label>
              Match
              <select
                value={subtree ? 'subtree' : 'exact'}
                onChange={(event) =>
                  setSubtree(event.target.value === 'subtree')
                }
              >
                <option value="subtree">Subtree</option>
                <option value="exact">Exact file</option>
              </select>
            </label>
            <button disabled={preferences.exclusions.length >= 64}>
              Add exclusion
            </button>
          </form>
          {validation && <p role="alert">{validation}</p>}
          <p className="secondary">
            {running
              ? 'Scan running. Edits apply only to the next scan; current scope stays fixed.'
              : 'Saved settings apply to your next explicit scan.'}
          </p>
          <Toolbar label="Save scan settings">
            <button
              type="button"
              onClick={() => void session.save()}
              disabled={state.saving}
            >
              {state.saving ? 'Saving…' : 'Save settings'}
            </button>
            <span>
              {state.dirty
                ? 'Unsaved changes'
                : preferences.configured
                  ? 'Saved locally'
                  : 'Start scan also saves these choices'}
            </span>
          </Toolbar>
        </>
      )}
    </section>
  );
}
