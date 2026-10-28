import { useMemo } from 'react';
import type { ReactElement } from 'react';
import type { Finding } from '../scan/contract';
import { formatBytes } from '../scan/accounting';
import { layoutTreemap } from './treemapLayout';

const labelMinimumWidth = 9;
const labelMinimumHeight = 14;

export function Treemap({
  findings,
  selectedId,
  onSelect,
  onOpen,
}: {
  findings: Finding[];
  selectedId: string | null;
  onSelect: (findingId: string) => void;
  onOpen: (findingId: string) => void;
}): ReactElement {
  const tiles = useMemo(() => layoutTreemap(findings), [findings]);
  const drawnIds = useMemo(
    () => new Set(tiles.map((tile) => tile.finding.id)),
    [tiles],
  );
  const notDrawn = findings.filter((finding) => !drawnIds.has(finding.id));
  return (
    <div className="treemap-region">
      <div
        className="treemap"
        role="group"
        aria-label="Treemap of allocated size"
      >
        {tiles.map(({ finding, left, top, width, height }) => (
          <button
            key={finding.id}
            type="button"
            className="treemap-tile"
            data-finding-id={finding.id}
            data-kind={finding.kind}
            data-partial={finding.complete ? undefined : 'true'}
            aria-pressed={selectedId === finding.id}
            aria-label={`Inspect ${finding.displayName}`}
            title={`${finding.displayName} · ${formatBytes(finding.allocatedBytes)}`}
            style={{
              left: `${left}%`,
              top: `${top}%`,
              width: `${width}%`,
              height: `${height}%`,
            }}
            onClick={() => onSelect(finding.id)}
            onDoubleClick={() => {
              if (finding.kind === 'directory') onOpen(finding.id);
            }}
          >
            {width >= labelMinimumWidth && height >= labelMinimumHeight && (
              <span className="treemap-label" aria-hidden="true">
                <strong className="path-text">{finding.displayName}</strong>
                <span>
                  {formatBytes(finding.allocatedBytes)}
                  {finding.complete ? '' : ' · partial'}
                </span>
              </span>
            )}
          </button>
        ))}
      </div>
      {tiles.length === 0 && (
        <p className="secondary">Nothing measurable to draw in this scope.</p>
      )}
      {notDrawn.length > 0 && (
        <p className="treemap-undrawn">
          {notDrawn.length} not drawn (unmeasured, shared or empty):{' '}
          {notDrawn
            .slice(0, 5)
            .map((finding) => finding.displayName)
            .join(', ')}
          {notDrawn.length > 5 ? '…' : ''}. Absence of a tile does not mean
          zero; use the list for each reason.
        </p>
      )}
    </div>
  );
}
