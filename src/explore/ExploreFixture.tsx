import type { ReactElement } from 'react';
import { ScanProvider } from '../scan/context';
import { ScanApplication } from '../scan/ScanApplication';
import {
  createFixtureApi,
  fixtureFindings,
  fixtureLocation,
  fixtureProgress,
} from '../scan/fixtures';
import { indexFindings } from './model';
import type { InspectionApi } from './contract';

const [root, file] = fixtureFindings();
if (!root || !file) throw new Error('Synthetic fixture missing');
const findings = [
  root,
  {
    ...root,
    id: '1:2',
    parentId: root.id,
    displayName: '開発 – SYNTHETIC project',
  },
  {
    ...file,
    id: '1:3',
    parentId: '1:2',
    displayName: '.hidden',
    complete: false,
    issue: 'permission_denied',
  },
  { ...file, id: '1:4', parentId: '1:2', displayName: 'same.txt' },
  {
    ...file,
    id: '1:5',
    parentId: root.id,
    displayName: 'same.txt',
    issue: 'vanished',
  },
  ...Array.from({ length: 125 }, (_unused, index) => ({
    ...file,
    id: `1:${index + 6}`,
    displayName: `SYNTHETIC-${index}.txt`,
    allocatedBytes: `${4096 * (index + 1)}`,
  })),
];
const index = indexFindings(findings);
const scanApi = createFixtureApi();
scanApi.current = async () => ({
  scope: { id: 'fixture', roots: [fixtureLocation], exclusions: [] },
  progress: fixtureProgress('partial'),
});
scanApi.page = async (_generation, offset) => ({
  progress: fixtureProgress('partial'),
  offset,
  nextOffset: null,
  findings,
});
const inspectionApi: InspectionApi = {
  inspect: async (_generation, findingId) => ({
    findingId,
    displayPath: index.paths.get(findingId) ?? 'Unavailable',
    copyPath: index.paths.get(findingId) ?? null,
    recognizedAtId: '1:2',
    artifact: {
      findingId: '1:2',
      family: 'javascript_project',
      nature: 'source',
      projectId: '1:2',
      confidence: 'high',
      evidence: [{ findingId: '1:4', method: 'SYNTHETIC manifest fixture' }],
      consequence:
        'SYNTHETIC source fixture. Removing source may destroy work.',
      complete: true,
      regenerable: false,
    },
  }),
  reveal: async () => {
    throw { code: 'unavailable' };
  },
  copy: (path) => navigator.clipboard.writeText(path),
};
export function ExploreFixture(): ReactElement {
  return (
    <ScanProvider api={scanApi}>
      <ScanApplication
        inspectionApi={inspectionApi}
        connectionStatus="SYNTHETIC Explore fixture · browser presentation only · Finder unavailable"
      />
    </ScanProvider>
  );
}
