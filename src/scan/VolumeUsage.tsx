import { useMemo } from 'react';
import type { ReactElement } from 'react';
import {
  categorize,
  categoryNames,
  formatBytes,
  isCappedBytes,
  segmentWidth,
} from './accounting';
import type { VolumeAccounting } from './contract';
import type { ScanSnapshot } from './session';
import { Sunburst } from './Sunburst';
import { buildSunburstCategories } from './sunburstGeometry';

function categoryIndex(name: (typeof categoryNames)[number]): number {
  return categoryNames.indexOf(name);
}

export function VolumeUsage({
  accounting,
  snapshot,
  partial,
  activeName,
  onActiveChange,
}: {
  accounting: VolumeAccounting;
  snapshot: ScanSnapshot;
  partial: boolean;
  activeName: string | null;
  onActiveChange: (name: string | null) => void;
}): ReactElement {
  const { volume } = accounting;
  const volumeFindings = useMemo(
    () =>
      snapshot.settled
        ? snapshot.findings.filter((finding) =>
            finding.identity?.startsWith(`${volume.id}:`),
          )
        : [],
    [snapshot.findings, snapshot.settled, volume.id],
  );
  const volumeCategories = useMemo(
    () => categorize(volumeFindings),
    [volumeFindings],
  );
  const categorizedBytes = volumeCategories.reduce(
    (total, category) => total + category.allocatedBytes,
    0n,
  );
  const matchingCategories =
    snapshot.settled &&
    categorizedBytes === BigInt(accounting.scopeAllocatedBytes);
  const total = volume.totalBytes;
  const remainder = accounting.unscannedOrUnexplainedBytes;
  const cappedAllocation =
    isCappedBytes(accounting.scopeAllocatedBytes) ||
    volumeCategories.some((category) => category.allocatedCapped);
  const cappedRemainder = cappedAllocation || isCappedBytes(volume.usedBytes);
  const cappedAccounting =
    cappedRemainder || [total, volume.freeBytes, remainder].some(isCappedBytes);
  const consistent =
    !cappedAccounting &&
    total !== null &&
    BigInt(total) > 0n &&
    volume.freeBytes !== null &&
    remainder !== null &&
    BigInt(accounting.scopeAllocatedBytes) +
      BigInt(remainder) +
      BigInt(volume.freeBytes) ===
      BigInt(total);
  const sunburstCategories = useMemo(
    () =>
      matchingCategories && consistent
        ? buildSunburstCategories(
            volumeCategories,
            categoryIndex,
            volumeFindings,
          )
        : [],
    [matchingCategories, consistent, volumeCategories, volumeFindings],
  );
  const measuredBytes = BigInt(accounting.scopeAllocatedBytes);
  const showChart = sunburstCategories.length > 0 && measuredBytes > 0n;
  return (
    <section className="volume-usage" aria-label={`Volume ${volume.id} usage`}>
      <div className="chart-card">
        {showChart ? (
          <>
            <Sunburst
              categories={sunburstCategories}
              measuredBytes={measuredBytes}
              usedLabel={
                volume.usedBytes === null
                  ? 'used space unavailable'
                  : `of ${formatBytes(volume.usedBytes)} used`
              }
              activeName={activeName}
              onActiveChange={onActiveChange}
            />
            <p className="chart-caption secondary">
              Inner ring: categories. Outer ring: largest files in each. Chart
              covers only measured space; unscanned or unexplained space is not
              shown as a category.
            </p>
          </>
        ) : (
          <div className="chart-placeholder">
            <p>
              {snapshot.settled
                ? 'No category chart for this volume: measured categories are empty or cannot be reconciled with volume accounting.'
                : 'Category chart appears after final measurements load. Ranking waits so shapes stay stable.'}
            </p>
          </div>
        )}
      </div>
      <div className="coverage-card">
        <div className="results-heading">
          <h3>{partial ? 'Coverage is partial' : 'Coverage as scanned'}</h3>
          <span>{formatBytes(total)} total</span>
        </div>
        {consistent &&
        total !== null &&
        remainder !== null &&
        volume.freeBytes !== null ? (
          <div className="usage-strip" aria-hidden="true">
            {matchingCategories ? (
              volumeCategories.map((category) => (
                <span
                  key={category.name}
                  className={`segment-category category-${categoryIndex(category.name)}`}
                  style={{
                    width: segmentWidth(
                      category.allocatedBytes.toString(),
                      total,
                    ),
                  }}
                />
              ))
            ) : (
              <span
                className="segment-measured"
                style={{
                  width: segmentWidth(accounting.scopeAllocatedBytes, total),
                }}
              />
            )}
            <span
              className="segment-unknown"
              style={{ width: segmentWidth(remainder, total) }}
            />
            <span
              className="segment-free"
              style={{ width: segmentWidth(volume.freeBytes, total) }}
            />
          </div>
        ) : (
          <p>
            {cappedAccounting
              ? 'Usage segments unavailable: capped accounting cannot reconcile exact volume usage.'
              : 'Usage segments unavailable: missing or inconsistent physical accounting.'}
          </p>
        )}
        {matchingCategories && (
          <p className="visually-hidden">
            Measured categories:{' '}
            {volumeCategories
              .filter((category) => category.allocatedBytes > 0n)
              .map(
                (category) =>
                  `${category.name} ${formatBytes(category.allocatedBytes, category.allocatedCapped)}`,
              )
              .join(' · ') || 'No allocated file contributions'}
            .
          </p>
        )}
        <dl className="volume-legend">
          <div>
            <dt>
              <span className="key-swatch key-measured" aria-hidden="true" />
              Measured allocated
            </dt>
            <dd>{formatBytes(accounting.scopeAllocatedBytes)}</dd>
          </div>
          <div>
            <dt>
              <span className="key-swatch key-unknown" aria-hidden="true" />
              Unscanned / unexplained · not measured
            </dt>
            <dd>
              {cappedRemainder
                ? 'Unavailable (capped accounting)'
                : formatBytes(remainder)}
            </dd>
          </div>
          <div>
            <dt>
              <span className="key-swatch key-free" aria-hidden="true" />
              Free blocks
            </dt>
            <dd>{formatBytes(volume.freeBytes)}</dd>
          </div>
          <div>
            <dt>Available to you</dt>
            <dd>{formatBytes(volume.availableBytes)}</dd>
          </div>
        </dl>
        <p className="secondary">
          Volume {volume.id} sampled{' '}
          {new Date(volume.measuredAtMs).toLocaleTimeString()}.{' '}
          {volume.metricIssue &&
            `Metric issue: ${volume.metricIssue.replaceAll('_', ' ')}.`}
        </p>
      </div>
    </section>
  );
}
