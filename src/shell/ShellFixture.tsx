import { useState } from 'react';
import type { ReactElement } from 'react';
import { AppShell } from './AppShell';
import { Dialog, Inspector, Notice, ResultTable, Toolbar } from './primitives';
import './fixture.css';

export function ShellFixture(): ReactElement {
  const [selected, setSelected] = useState(false);
  const [inspecting, setInspecting] = useState(false);
  const [confirming, setConfirming] = useState(false);
  return (
    <AppShell
      connectionStatus="Synthetic shell fixture · No files accessed"
      views={{
        explore: (
          <>
            <Notice>
              Synthetic fixture data — presentation QA only. No files accessed
              or changed.
            </Notice>
            <Toolbar label="Fixture actions">
              <button type="button" onClick={() => setConfirming(true)}>
                Open fixture dialog
              </button>
              <button
                type="button"
                onClick={() =>
                  document.documentElement.classList.toggle(
                    'fixture-text-enlarged',
                  )
                }
              >
                Toggle 200% fixture text
              </button>
            </Toolbar>
            <div className="workspace-split">
              <div>
                <label>
                  Fixture search{' '}
                  <input aria-label="Fixture search" type="search" />
                </label>
                <ResultTable caption="Synthetic items">
                  <thead>
                    <tr>
                      <th scope="col">Select</th>
                      <th scope="col">Name</th>
                      <th scope="col" className="numeric">
                        Logical size
                      </th>
                      <th scope="col">Details</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr>
                      <td>
                        <input
                          type="checkbox"
                          aria-label="Select synthetic project"
                          checked={selected}
                          onChange={(event) =>
                            setSelected(event.target.checked)
                          }
                        />
                      </td>
                      <td>
                        Synthetic project with a long Unicode name — پروژه
                        آزمایشی
                      </td>
                      <td className="numeric">12 MiB</td>
                      <td>
                        <button
                          type="button"
                          onClick={() => setInspecting(true)}
                        >
                          Inspect fixture
                        </button>
                      </td>
                    </tr>
                  </tbody>
                </ResultTable>
                <p>
                  {selected
                    ? '1 synthetic item selected'
                    : 'No synthetic items selected'}
                </p>
                {Array.from({ length: 12 }, (_, index) => (
                  <p key={index}>
                    Synthetic scroll fixture row {index + 1}. No filesystem
                    data.
                  </p>
                ))}
              </div>
              <Inspector
                title="Synthetic project"
                open={inspecting}
                onClose={() => setInspecting(false)}
              >
                <p>
                  Fixture only. Ownership and removal consequences unavailable.
                </p>
              </Inspector>
            </div>
            <Dialog
              title="Synthetic dialog"
              open={confirming}
              onClose={() => setConfirming(false)}
            >
              <p>Focus and Escape fixture. No operation will run.</p>
            </Dialog>
          </>
        ),
      }}
    />
  );
}
