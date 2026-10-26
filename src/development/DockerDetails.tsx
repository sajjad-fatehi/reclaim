import type { ReactElement } from 'react';
import { formatBytes } from '../scan/accounting';
import { Notice } from '../shell/primitives';
import type { DockerItem } from './contract';

export function DockerDetails({
  item,
  measuredAtMs,
}: {
  item: DockerItem;
  measuredAtMs: string | null;
}): ReactElement {
  function stateLabel(value: boolean | null): string {
    return value === null ? 'Unknown' : value ? 'Yes' : 'No';
  }
  return (
    <div className="item-inspection">
      <h3 className="path-text">Docker {item.kind.replaceAll('_', ' ')}</h3>
      <p className="path-text">{item.identity}</p>
      <Notice>
        Inspection only. Docker observations are separate from filesystem
        accounting; shared layers and VM storage are not additive or guaranteed
        recoverable.
      </Notice>
      <dl className="inspection-facts">
        <dt>Observed</dt>
        <dd>
          {measuredAtMs && Number.isSafeInteger(Number(measuredAtMs))
            ? new Date(Number(measuredAtMs)).toLocaleString()
            : 'Unknown'}
        </dd>
        <dt>Logical observation</dt>
        <dd>{formatBytes(item.logicalBytes)}</dd>
        <dt>Shared bytes</dt>
        <dd>{formatBytes(item.sharedBytes)}</dd>
        <dt>Metric basis</dt>
        <dd>{item.metricBasis}</dd>
        <dt>Active</dt>
        <dd>{stateLabel(item.active)}</dd>
        <dt>Persistent data</dt>
        <dd>{stateLabel(item.persistent)}</dd>
        <dt>Shared</dt>
        <dd>{stateLabel(item.shared)}</dd>
        <dt>References</dt>
        <dd>{item.references ?? 'Unknown'}</dd>
        <dt>Confidence / coverage</dt>
        <dd>
          {item.artifact.confidence} ·{' '}
          {item.artifact.complete
            ? 'complete observation'
            : 'partial observation'}
        </dd>
        <dt>Related projects</dt>
        <dd>
          Unknown. Inventory does not establish filesystem project ownership.
        </dd>
      </dl>
      <h3>Removal consequences</h3>
      <p>{item.artifact.consequence}</p>
      <h3>Recognition evidence</h3>
      <ul>
        {item.artifact.evidence.map((evidence, index) => (
          <li key={`${evidence.findingId}:${index}`} className="path-text">
            {evidence.method} · {evidence.findingId}
          </li>
        ))}
      </ul>
    </div>
  );
}
