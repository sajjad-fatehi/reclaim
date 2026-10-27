import type { ReactElement } from 'react';
import { formatBytes } from './accounting';
import { describeArc, layoutArcs } from './sunburstGeometry';
import type { SunburstCategory } from './sunburstGeometry';

const viewSize = 420;
const center = viewSize / 2;
const categoryRing = { inner: 104, outer: 152 };
const itemRing = { inner: 157, outer: 200 };

export function Sunburst({
  categories,
  measuredBytes,
  usedLabel,
  activeName,
  onActiveChange,
}: {
  categories: SunburstCategory[];
  measuredBytes: bigint;
  usedLabel: string;
  activeName: string | null;
  onActiveChange: (name: string | null) => void;
}): ReactElement {
  const categoryArcs = layoutArcs(
    categories.map((category) => category.bytes),
    measuredBytes,
  );
  const active = categories.find((category) => category.name === activeName);
  return (
    <svg
      className="sunburst"
      viewBox={`0 0 ${viewSize} ${viewSize}`}
      aria-hidden="true"
      focusable="false"
      data-active={active ? 'true' : 'false'}
    >
      {categories.map((category, categoryPosition) => {
        const categoryArc = categoryArcs[categoryPosition];
        if (!categoryArc) return null;
        const itemArcs = layoutArcs(
          category.items.map((item) => item.bytes),
          category.bytes,
          categoryArc.startAngle,
          categoryArc.endAngle - categoryArc.startAngle,
        );
        return (
          <g
            key={category.name}
            className={`sunburst-category category-${category.index}`}
            data-dimmed={
              activeName !== null && activeName !== category.name
                ? 'true'
                : 'false'
            }
            style={{ ['--arc-index' as string]: categoryPosition }}
            onMouseEnter={() => onActiveChange(category.name)}
            onMouseLeave={() => onActiveChange(null)}
          >
            <path
              className="sunburst-arc"
              fillRule="evenodd"
              d={describeArc(
                categoryRing.inner,
                categoryRing.outer,
                categoryArc,
                center,
              )}
            />
            {category.items.map((item, itemPosition) => {
              const itemArc = itemArcs[itemPosition];
              if (!itemArc) return null;
              return (
                <path
                  key={item.id}
                  className="sunburst-arc sunburst-item"
                  fillRule="evenodd"
                  d={describeArc(
                    itemRing.inner,
                    itemRing.outer,
                    itemArc,
                    center,
                  )}
                />
              );
            })}
          </g>
        );
      })}
      <g className="sunburst-label" textAnchor="middle">
        <text x={center} y={center - 30} className="sunburst-label-caption">
          {active ? active.name : 'Measured in scope'}
        </text>
        <text x={center} y={center + 6} className="sunburst-label-figure">
          {formatBytes(active ? active.bytes : measuredBytes)}
        </text>
        <text x={center} y={center + 32} className="sunburst-label-detail">
          {active ? 'allocated in this category' : usedLabel}
        </text>
      </g>
    </svg>
  );
}
