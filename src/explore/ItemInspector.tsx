import { useEffect, useRef, useState } from 'react';
import type { ReactElement } from 'react';
import type { Finding, Progress } from '../scan/contract';
import { formatBytes } from '../scan/accounting';
import { Inspector, Notice, Toolbar } from '../shell/primitives';
import { actionError, nativeInspectionApi } from './contract';
import type { Inspection, InspectionApi } from './contract';
import { measurementStatus } from './model';
import type { DockerItem } from '../development/contract';
import { DockerDetails } from '../development/DockerDetails';
import { ReviewButton } from '../cleanup/ReviewButton';

export interface ItemInspectorProps {
  finding: Finding | null;
  dockerItem?: DockerItem | null | undefined;
  dockerMeasuredAtMs?: string | null | undefined;
  progress: Progress;
  stale: boolean;
  settled: boolean;
  focusOnChange?: boolean;
  onClose: () => void;
  onInspectRelated: (findingId: string) => void;
  api?: InspectionApi | undefined;
}
const kindLabels: Record<Finding['kind'], string> = {
  directory: 'Folder',
  file: 'File',
  symlink: 'Symlink',
  other: 'Other',
  unknown: 'Unknown type',
};
function timeLabel(timestamp: number | null): string {
  return timestamp === null ? 'Unknown' : new Date(timestamp).toLocaleString();
}
export function ItemInspector(props: ItemInspectorProps): ReactElement {
  const contentReference = useRef<HTMLDivElement>(null);
  const previousSelection = useRef<string | null>(null);
  const selectionKey = props.finding
    ? `${props.progress.generation}:${props.finding.id}`
    : (props.dockerItem?.artifact.findingId ?? null);
  useEffect(() => {
    if (
      (props.focusOnChange ?? true) &&
      previousSelection.current !== null &&
      selectionKey !== null &&
      previousSelection.current !== selectionKey
    )
      contentReference.current?.focus();
    previousSelection.current = selectionKey;
  }, [selectionKey, props.focusOnChange]);
  return (
    <Inspector
      title="Item details"
      open={props.finding !== null || !!props.dockerItem}
      onClose={props.onClose}
    >
      {props.dockerItem && (
        <div
          ref={contentReference}
          tabIndex={-1}
          role="group"
          aria-label={`Docker ${props.dockerItem.identity}`}
        >
          <DockerDetails
            item={props.dockerItem}
            measuredAtMs={props.dockerMeasuredAtMs ?? null}
          />
          <ReviewButton
            candidate={
              props.dockerItem.kind === 'image' &&
              props.dockerItem.references === 0
            }
            selection={{
              source: 'docker',
              findingId: props.dockerItem.artifact.findingId,
            }}
          />
        </div>
      )}
      {props.finding && (
        <div
          className="item-inspection"
          ref={contentReference}
          tabIndex={-1}
          role="group"
          aria-label={props.finding.displayName}
        >
          <InspectorContent
            key={selectionKey}
            {...props}
            finding={props.finding}
          />
        </div>
      )}
    </Inspector>
  );
}
function InspectorContent({
  finding,
  progress,
  stale,
  settled,
  onInspectRelated,
  api = nativeInspectionApi,
}: ItemInspectorProps & { finding: Finding }): ReactElement {
  const [details, setDetails] = useState<Inspection | null>(null);
  const [message, setMessage] = useState('');
  const [busy, setBusy] = useState(false);
  const [revision, setRevision] = useState(0);
  const generation = progress.generation;
  const findingId = finding?.id;
  useEffect(() => {
    let active = true;
    if (!findingId) return;
    void api
      .inspect(generation, findingId)
      .then((inspection) => {
        if (!active) return;
        if (inspection.findingId !== findingId) {
          setMessage('Details belong to another item. Retry details.');
          return;
        }
        setDetails(inspection);
      })
      .catch((error: unknown) => {
        if (active) setMessage(actionError(error));
      });
    return () => {
      active = false;
    };
  }, [api, generation, findingId, settled, revision]);
  async function perform(action: 'copy' | 'reveal'): Promise<void> {
    if (!finding || busy) return;
    setBusy(true);
    setMessage('');
    try {
      if (action === 'copy') {
        if (details?.copyPath === null || !details) return;
        await api.copy(details.copyPath);
      } else await api.reveal(generation, finding.id);
      setMessage(action === 'copy' ? 'Path copied.' : 'Sent to Finder.');
    } catch (error) {
      setMessage(actionError(error));
    } finally {
      setBusy(false);
    }
  }
  const artifact = details?.artifact;
  const subtitle = [
    kindLabels[finding.kind],
    artifact?.family.replaceAll('_', ' '),
    artifact ? `${artifact.confidence} confidence` : null,
  ]
    .filter(Boolean)
    .join(' · ');
  return (
    <>
      <header className="inspector-hero" data-kind={finding.kind}>
        <h3 className="path-text">{finding.displayName}</h3>
        <p>{subtitle}</p>
      </header>
      <p className="inspector-figure">
        <strong>{formatBytes(finding.allocatedBytes)}</strong>
        <span>
          allocated · {formatBytes(finding.logicalBytes)} logical
          {finding.complete ? '' : ' · partial'}
        </span>
      </p>
      <h3 className="inspector-label">Path</h3>
      <p className="path-box path-text">
        {details?.displayPath ?? 'Full path loading…'}
      </p>
      <p className="secondary path-text">
        Finding {finding.id} · generation {generation}
      </p>
      <Toolbar label="Item actions">
        <ReviewButton
          candidate={
            details?.recognizedAtId === finding.id &&
            !!artifact &&
            (artifact.regenerable || artifact.nature === 'worktree')
          }
          disabled={stale || !settled}
          selection={{
            source: 'filesystem',
            generation,
            findingId: finding.id,
          }}
        />
        <button
          disabled={busy || stale || finding.kind === 'symlink'}
          onClick={() => void perform('reveal')}
        >
          Show in Finder
        </button>
        <button
          disabled={busy || !details || details.copyPath === null}
          onClick={() => void perform('copy')}
        >
          Copy path
        </button>
        <button
          disabled={busy}
          onClick={() => {
            setDetails(null);
            setMessage('');
            setRevision((value) => value + 1);
          }}
        >
          Retry details
        </button>
      </Toolbar>
      {details?.copyPath === null && (
        <Notice>
          Path contains non-UTF-8 bytes. Display uses escaping; exact text copy
          is unavailable. Native identity still supports Finder.
        </Notice>
      )}
      <p role="status">{message}</p>
      <h3 className="inspector-label">Consequences</h3>
      <p>
        {artifact?.consequence ??
          'Removal consequences unknown. This may contain source, personal or persistent data. Inspection does not authorize removal.'}
      </p>
      <p className="secondary">
        Allocated contribution is not guaranteed recoverable space; APFS sharing
        and snapshots remain unknown. Inspection only. Review cleanup options
        explains native eligibility; selection does not authorize removal.
        Modification time does not prove inactivity; growth history and physical
        recovery are unknown.
      </p>
      <h3 className="inspector-label">Ownership and evidence</h3>
      {artifact ? (
        <>
          <p>
            {artifact.family.replaceAll('_', ' ')} ·{' '}
            {artifact.nature.replaceAll('_', ' ')} · {artifact.confidence}{' '}
            confidence
          </p>
          {details.recognizedAtId !== finding.id && (
            <p>
              Inherited from recognized ancestor; this item was not
              independently classified.
            </p>
          )}
          <ul className="evidence-list">
            {artifact.evidence.map((evidence) => (
              <li key={`${evidence.findingId}:${evidence.method}`}>
                <button onClick={() => onInspectRelated(evidence.findingId)}>
                  Inspect evidence: {evidence.method.replaceAll('_', ' ')}
                </button>
              </li>
            ))}
          </ul>
          {artifact.projectId && (
            <button onClick={() => onInspectRelated(artifact.projectId ?? '')}>
              Inspect related project
            </button>
          )}
          {details.recognizedAtId && details.recognizedAtId !== finding.id && (
            <button
              onClick={() => onInspectRelated(details.recognizedAtId ?? '')}
            >
              Inspect recognized owner/tool
            </button>
          )}
          {!artifact.complete && (
            <Notice>Recognition incomplete; ownership may be uncertain.</Notice>
          )}
        </>
      ) : (
        <p>
          {details
            ? 'Unknown owner. No recognized evidence for this item.'
            : 'Ownership unavailable until details load.'}
        </p>
      )}
      <h3 className="inspector-label">Measurement</h3>
      <dl className="inspection-facts">
        <dt>Measured</dt>
        <dd>{timeLabel(finding.measuredAtMs)}</dd>
        <dt>Modified</dt>
        <dd>{timeLabel(finding.modifiedAtMs)}</dd>
        <dt>Coverage</dt>
        <dd>
          {measurementStatus(finding)} · scan {progress.status}
          {!settled ? ' · updating' : ''}
          {stale ? ' · stale' : ''}
        </dd>
      </dl>
      <p className="secondary">
        Directory totals include measured descendants. Shared identities count
        once{finding.aliasOf ? `; counted at ${finding.aliasOf}` : ''}.
      </p>
    </>
  );
}
