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
  createRoot(rootContainer).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}

void renderApplication(container);
