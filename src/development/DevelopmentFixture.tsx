import type { ReactElement } from 'react';
import { ScanProvider } from '../scan/context';
import { ScanApplication } from '../scan/ScanApplication';
import { developmentFixture } from './fixtures';

const fixture = developmentFixture();
export function DevelopmentFixture(): ReactElement {
  return (
    <ScanProvider api={fixture.scanApi}>
      <ScanApplication
        developmentApi={fixture.api}
        inspectionApi={fixture.inspectionApi}
        connectionStatus="SYNTHETIC Development fixture · no native scanning, Docker or file operations"
      />
    </ScanProvider>
  );
}
