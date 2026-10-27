import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { App } from './App';
import './styles.css';

const container = document.getElementById('root');

if (!container) {
  throw new Error('Application root element is missing');
}

async function renderApplication(rootContainer: HTMLElement): Promise<void> {
  if (
    import.meta.env.DEV &&
    new URLSearchParams(location.search).get('fixture') === 'shell'
  ) {
    const { ShellFixture } = await import('./shell/ShellFixture');
    createRoot(rootContainer).render(
      <StrictMode>
        <ShellFixture />
      </StrictMode>,
    );
    return;
  }
  if (
    import.meta.env.DEV &&
    new URLSearchParams(location.search).get('fixture') === 'explore'
  ) {
    const { ExploreFixture } = await import('./explore/ExploreFixture');
    createRoot(rootContainer).render(
      <StrictMode>
        <ExploreFixture />
      </StrictMode>,
    );
    return;
  }
  if (
    import.meta.env.DEV &&
    new URLSearchParams(location.search).get('fixture') === 'development'
  ) {
    const { DevelopmentFixture } =
      await import('./development/DevelopmentFixture');
    createRoot(rootContainer).render(
      <StrictMode>
        <DevelopmentFixture />
      </StrictMode>,
    );
    return;
  }
  if (
    import.meta.env.DEV &&
    new URLSearchParams(location.search).get('fixture') === 'cleanup'
  ) {
    const { CleanupFixture } = await import('./cleanup/CleanupFixture');
    createRoot(rootContainer).render(
      <StrictMode>
        <CleanupFixture />
      </StrictMode>,
    );
    return;
  }
  createRoot(rootContainer).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}

void renderApplication(container);
