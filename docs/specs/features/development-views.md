# Development: Projects and Tools

Follow [shared requirements](index.md), the shell, shared inspector, scan-session and recognition/Docker contracts. Do not start independent scans or reimplement recognizers in React.

## Required behavior

- Projects and Tools are two local tabs under Development. Both show real measured/recognized findings with scoped filters, comparable sizes, coverage and sort state.
- Project detail shows source versus dependencies/build/logs and linked worktrees. Expose branch/change/lock/unknown state without claiming all worktrees can be removed.
- Tool detail groups shared package stores, caches/runtimes, editors/coding tools and local Docker inventory. Show related projects and shared ownership without attributing the full same store to every project.
- Use the shared inspector for evidence, consequences, full paths/IDs and recognition confidence. Keep unknown/unsupported resources inspectable and distinguish missing tool, denied scope and no matching artifacts.
- Cross-links to Explore preserve scope and selected item. Suggestions explain actual evidence; no arbitrary health scores or reclaimable amounts derived from age.
- Integrate icon language, linked selection and bounded motion consistently. Add-to-review wiring is delivered by cleanup; do not ship dummy enabled mutation controls.

## Acceptance and proof

- End-to-end fixture sessions render each recognized family and Docker states, project/tool cross-links, shared storage counted once, worktree protections and partial/error/empty states.
- Navigation/tab/filter/selection preserves context; keyboard/VoiceOver, long names, compact windows, both themes and reduced motion remain usable.
- Relevant frontend/contract tests and required checks pass; attach native fixture screenshots. No scanner duplication, runtime install, daemon startup or cleanup is performed.

## Implementation contract (recorded before edits)

Development consumes the existing shared scan session. After its filesystem snapshot
settles, `recognition_page({generation, offset, limit: 200})` supplies native artifacts,
exclusive contributions, owned project totals, worktree metadata and diagnostics.
React groups and filters these observations; it never recognizes paths or calculates
cleanup eligibility. Project totals exclude nested projects; exclusive artifact rows
exclude separately recognized descendants. Neither is added to the other. Shared
stores remain one artifact with unknown consumers unless native project IDs provide
attribution. Similar tool families do not prove a project uses a shared store.

Projects includes native project/worktree roots, source remainder, owned artifacts
and repository-ID-linked worktrees. Tools presents every artifact family, grouped by
native family, including unassigned/protected data. Scoped search, family/nature
filters, size/name sorting and 50-row pages retain independent tab state and selection
through shell navigation. Generation replacement resets obsolete identities.
Cross-links send generation, exact finding IDs and selected ID to Explore while
retaining its prior browsing context. Filesystem details reuse `ItemInspector`;
Docker observations extend the same inspector with IDs, evidence, active/persistent/
shared state, metric basis and consequences, without filesystem actions.

Docker inventory is an explicit user action, using delivered `docker_inventory` and
`docker_cancel`; no daemon startup, installation or deletion occurs. Its separately
timestamped observations never enter filesystem/project totals. Unavailable, denied,
unsupported, unknown and empty inventory states stay distinct. Scan and recognition
partial/cancelled/limited/error/stale states remain visible, with explicit retry.

Verification covers strict transport decoding, generation/pagination guards, all
recognition families, native totals retained without duplication, project/worktree
links, filters/sort/pages/selection, Explore handoff, Docker failure and safety states,
and required repository gates. Native synthetic fixture captures, keyboard, VoiceOver,
light/dark, compact/enlarged text and reduced motion are attempted through supported
UI tooling; missing evidence is reported explicitly.

## Delivered contract and evidence

- `src/development/Development.tsx` mounts in the existing `ScanApplication` shell.
  Independent Projects/Tools tab filters, sorting, pages and selected IDs survive
  shell navigation. Arrow/Home/End tab keys, labelled native search/select controls,
  sortable semantic tables and the existing responsive inspector preserve keyboard
  semantics. Family groups use distinct table row groups. Each table page renders
  at most 50 findings; full IDs/paths and Unicode text remain available.
- `contract.ts` decodes the actual native recognition and Docker DTOs, rejecting
  malformed states/byte counts, stale generations, duplicate finding IDs and invalid
  continuation offsets. Native's shared 100 ms read throttle gets one bounded
  120 ms retry. No native scanner/recognizer, permissions or mutation boundary changed.
- Project rows use native `ownedAllocatedBytes`; detail lists native exclusive source
  remainder, dependencies/build/logs and repository-ID-linked worktrees. Branch,
  detached, main/linked, dirty/untracked/ignored/clean/unknown and lock states come
  directly from native metadata. Missing metadata never claims an unlocked worktree.
- Tools includes every delivered family and nature, including shared stores, runtime,
  editor/coding-tool state, temporary and protected data. Native project IDs link
  artifacts to projects. Shared consumers remain explicitly unknown; neither family
  similarity nor path presentation invents attribution. No frontend byte aggregation
  or reclaimability calculation exists.
- Explicit Docker inspect/refresh/cancel actions use delivered fixed native commands.
  Logical observations, shared bytes, references, active/unknown and persistent state,
  timestamps, confidence, metric basis, evidence and consequences use the shared
  `ItemInspector`. Docker resources use native artifact IDs, since raw resource IDs
  may overlap across kinds. They never get filesystem/Finder/cleanup controls or
  enter filesystem totals. Missing CLI, unsupported context/version/engine, denied,
  unavailable daemon, limited, malformed, timeout, cancelled, empty and failed
  transport observations remain distinguishable. Retained observations are labelled
  stale during refresh or transport failure.
- Explore handoff carries exact generation/finding/selected IDs through existing
  `ScanSession.investigate`; Reset filters restores prior browsing scope. Closing and
  repeating the same handoff reopens selection. Hidden shell destinations close
  their inspector presentation while retaining selection, avoiding hidden modal
  focus traps. Shared inspector ownership/path/reveal authorization is unchanged.
- Development-only `/?fixture=development` injects explicitly labelled synthetic
  scan, recognition, inspection and Docker APIs. It never scans files, contacts Docker
  or calls Finder. It covers all 24 native families, source/generated relationships,
  shared attribution unknowns and protected linked-worktree state. Production entry
  remains the native provider; fixture routing is guarded by `import.meta.env.DEV`.

### Automatic checks

- `rtk npm run check`: passed format, ESLint, strict TypeScript, 44 frontend tests,
  workspace Clippy and 50 Rust tests (28 core + 22 desktop). Initial gate found a
  missing test-helper option annotation; corrected before this passing gate.
- `rtk npx vitest run src/development/development.test.tsx src/explore/explore.test.tsx`:
  20 tests passed after semantic row-group and inspector-focus edits. Development's
  12 tests cover strict native DTOs/all families, pagination/throttle/generation
  guards, untouched native totals, worktree links/protections, 50-row rendering,
  tab keyboard/filter/selection state, exact/repeated Explore handoffs, Docker
  explicit actions, active/persistent/unknown metrics, failure/empty/cancel/late results.
- One Impeccable detector pass found only pre-existing `src/styles.css` advisory
  ramp/radius values (1.125rem, 1.75rem, 2px), outside these edits. Existing warm
  tokens/components remain authoritative; no unrelated design drift repair.

### Accepted capture limitation

Formal finish-review disposition: **recapture**. No current Development screenshot
exists. Missing evidence: standard light/dark Projects/Tools/inspector, compact
`900×600`, `200%` text, reduced-motion rendering, native keyboard and VoiceOver.
Source semantics and component focus tests do not prove native spoken output,
rendered contrast, clipping or motion behavior. Prior Overview/Explore screenshots
are not Development evidence.

`npx tauri dev --config /tmp/reclaim-development-qa.json` compiled/launched the native
synthetic-only Development fixture, reusing Vite on `localhost:1420`. CUA rejected its
executable with `Invalid app`; running-app inventory omitted Reclaim and browser
inventory was empty. Manager accepted these missing checks in request
`2d6ffa13-fe39-4afc-93c1-40dd700dc1ab` and prohibited further attachment attempts or
bypass. Issue `0096a2d5-10cd-4817-8f9d-e6b09430a1aa` retains the unresolved environment
limit. No native interaction pass, fabricated screenshot or unconditional visual
ship verdict is claimed. No real file scan, Docker inventory/deletion or cleanup was
performed for this delivery.

Final frontend evidence: `rtk npm run build:web` passed strict TypeScript and Vite
production bundling (88 modules; main JS 104.98 kB gzip). Focused Development tests
and changed-file Prettier/ESLint checks passed after the final Docker error-copy edit;
unchanged Rust and full-suite evidence above is reused. Shared Docker selection-focus
behavior is also covered by the combined 20-test Development/Explore run.
