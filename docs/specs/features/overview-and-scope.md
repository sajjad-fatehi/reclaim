# First run, scan settings and Overview

Follow [shared requirements](index.md) and [interaction flows](../../design/interaction-flows.md). Consume the delivered scanner and shell contracts; do not invent parallel ones.

## Required behavior

- First run shows editable suggested locations, purpose and access state. Add locations through the actual native picker, remove suggestions and explicitly Start scan. No traversal before that action.
- Settings manages scan locations, exclusions and access explanations; use the shell's appearance settings. Persist preferences locally with validated atomic writes and safe corrupt-file handling. Persisting a path does not grant fresh consent to scan on launch.
- Exclusions have clear exact/subtree semantics and visible coverage effects. Handle duplicate, nested, missing and inaccessible roots, new/removed volumes and changed permissions. Apply scope changes to the next scan; do not silently alter an active scan.
- Own one frontend scan-session orchestration layer consumed by completed sections. Surface stage/location/elapsed time, measured findings, cancellation and completed/partial/failed states. Do not invent percent completion when total work is unknown or continuously reorder interacting lists.
- Overview shows real volume totals, measurement time, scope coverage, segmented usage and matching ranked categories. Keep unscanned/unexplained/unavailable values explicit; no fake reconciled totals or inflated reclaim estimates.
- Category selection and reasoned investigation suggestions open corresponding Explore/Development context when available. Use the signature follow-the-space relationship and accessible textual equivalents.
- Refresh results as a deliberate scan generation; label retained older data and handle volume/disconnection/error changes without losing usable partial results.

## Acceptance and proof

- Native end-to-end fixture flow: suggested scope → edit/exclude → explicit start → progressive results → Overview; restart preserves settings but performs no auto-scan.
- Cancel, no readable roots, no matches, partial permission failure, corrupt settings and stale generation each render distinct actionable states.
- Fixture categories and volume/scope numbers are truthful and do not double-count. An unknown metric is not displayed as zero.
- Focused Rust preference and React session/state tests, relevant repository checks and a native UI smoke pass. Show actual fixture screenshots; never scan the user's private folders to populate a showcase.
- All copy is English; keyboard/VoiceOver/status cadence, both themes and reduced motion meet the shared UI contract.

## Implementation contract (before edits)

- Extend the delivered descriptor scanner, native scope registry and shell slots.
  Keep one React scan provider; consumers never create another polling loop.
- One native `preferences.json` owns appearance, selected native paths and exclusions.
  Bounded strict schema; mode 0600 temporary file, fsync and atomic rename; corrupt
  files remain untouched until an explicit save. A failed save retains session settings.
  Saved paths become reviewable candidates only, never scan consent. Missing roots
  remain visible and removable; native folder reselection refreshes authority.
- Exclusions are relative to every selected root, case-sensitive and literal (no glob).
  Exact-file rules skip only that file, leaving directories traversable. Subtree
  rules skip the named entry and everything beneath it. Reject absolute paths,
  dot/parent components, control characters and oversized lists. Rules are frozen
  in each registered scope and applied before child metadata/traversal. Excluded
  entries carry explicit coverage issues; totals contain measured contributions only.
- Retain prior generation separately during refresh. Serialize bounded page polling
  at >=150ms, re-read terminal pages to finalize mutable directory aggregates, reject
  late generation replies. Show provisional ingestion and retained/stale data explicitly.
- Overview categories partition unique regular-file contributions by file extension
  (Documents, Media, Archives, Other files); no source/artifact recognition inference.
  Categories rank only after terminal ingestion; volume segments use native allocated
  accounting, unknown metrics remain unavailable. Category investigation targets are
  typed context for future Explore consumers; unavailable destinations explain status.
- Native smoke uses isolated synthetic directories and the actual WebView/IPC.
  React fixtures cover edit/exclusion/start/cancel, restore-without-start, empty/denied,
  corrupt configuration and stale generation. Required gate: `npm run check`.

## Delivered shared frontend contract

`src/scan/contract.ts` mirrors and decodes native DTOs; all bytes stay decimal
strings and accounting uses `bigint`. `ScanProvider` in `src/scan/context.tsx`
wraps the app once. Consumers import `useScanSession` from
`src/scan/useScanSession.ts` and consume `{ state, session }`:

- `state.snapshot`: `{ scope, progress, findings, settled }`, one authoritative
  generation; finding IDs/parent IDs retain the native hierarchy. `settled` means
  terminal pages have been reloaded, not just that a terminal status was observed.
- `state.previous`: retained previous generation during refresh, explicitly stale.
  It is never mixed into current categories or navigation authority.
- `state.stale`, `error`, `loading`, `busy`, `saving`, `cancelling`, `dirty`:
  presentation/transport state. Scope changes never change `snapshot.scope`.
- `state.preferences`: editable next-scan roots, literal exclusions and appearance.
  Missing stored roots have `location: null`; retain or remove them, or pick again.
- `session.start()`, `cancel()`, `resume()`, `pickFolder()`, `editPreferences()`
  and `save()`: the single orchestration owner. Consumers must not poll separately;
  the native 100ms read budget covers all retrievals within the session.
- `state.investigation`: `{ generation, category, findingIds }`, set by category
  selection, then routed to Explore. It is a filter request, never permission or
  cleanup eligibility. Explore currently explains its pending implementation;
  consumers replace that shell slot while preserving this provider.

Pages are serialized every 160ms. Rate limits retry after that delay. Terminal
results receive a full second pass to refresh mutable ancestor aggregates.
Out-of-generation replies fail closed and retain usable older observations. Other
transport failures pause polling and expose Retry connection. Reopening reconnects
through `scan_current`; it never calls `scan_start`.

Categories partition non-alias regular-file contributions by literal extension:
Documents, Media, Archives and Other files. No directories are added again. Ranking
waits until final ingestion; allocations and logical sizes remain separate. The
largest measured category provides the investigation reason. Development-specific
recommendations await the recognition/Development producer, rather than inferring
project ownership from extensions. Unavailable metric values display `Unavailable`.
The volume strip renders only if free + measured allocated + unexplained equals
native total; otherwise it explains inconsistent/unavailable accounting.

## Native configuration and exclusion extension

`src-tauri/src/preferences.rs` owns one local `preferences.json` in Tauri's app
configuration directory. Appearance now shares that owner with scan settings;
the standalone shell fixture retains its browser-storage implementation. Installed
`serde_json` and `tempfile` moved from dev dependencies to runtime dependencies for
strict JSON decoding and safe atomic file replacement; no new packages were added.

- `preferences_load {}` returns `{ appearance, roots, exclusions, configured,
  corrupt }`. Each root is `{ key, displayPath, location }`; `location` is a fresh
  candidate or null when unavailable. No traversal or scan starts. Persisted paths
  stay native; the frontend sees only bounded display paths and candidate IDs.
- `preferences_save { update: { appearance, roots: [{key, locationId}],
  exclusions } }` resolves current candidate IDs or retained native preference
  keys; arbitrary frontend filesystem paths are never accepted. Bounds: 16 roots,
  64 exclusions, 1024 bytes per relative exclusion, 128KiB configuration read.
  Save validates before replacing a 0600 fsynced temporary file; invalid/corrupt
  files are preserved until an explicit save. Errors omit OS/private details.
- `scan_register_scope` adds optional `exclusions: { relativePath, subtree }[]`.
  Existing callers omitting it keep empty exclusions. `Scope.exclusions` freezes
  rules. Core `scan_excluding` extends the existing scanner, preserving its
  descriptor anchoring, bounds, hard-link accounting and cancellation.
- `Issue` adds `excluded`; intentional exclusions result in partial coverage, with
  unknown descendant contents. A subtree is skipped before reading its metadata;
  exact-file rules never stop directory traversal. Relative rules apply to every
  selected root; exclusions from overlapping requested roots remain effective
  after root normalization. No wildcard or modification-time inference exists.
- `Progress.currentLocation` identifies the currently measured root, not a guessed
  per-file cursor. Time is elapsed wall time; status announcements occur on phase
  changes, while elapsed time remains outside the live status region.

Native preferences are untrusted data and restore reviewable candidates only.
Every scan still requires explicit scope registration and Start scan; changed root
identities fail the existing native validation. Missing volumes/access never become
zero-sized successful scans.

## Verification evidence (October 27, 2026)

| Acceptance | Observed proof |
| --- | --- |
| Scope/exclusions/explicit start | Native macOS production React/WebView fixture: remove large suggestion, actual native picker reselect small fixture, add literal `cache` subtree, Save settings retains Not started, then Start scan |
| Truthful partial Overview | Four observed entries, one excluded subtree, two unique regular files: 32,773 logical bytes; Media 32KiB allocated + Documents 4KiB allocated = 36KiB allocated; excluded 1KiB contents never enumerated |
| Real volume accounting | Native descriptor volume totals and checked remainder rendered; category contributions partition per-volume measured allocation when final pages match; textual unavailable/inconsistent fallback tested |
| Progressive/cancel/retained data | Actual native UI showed 5,288 entries running; Cancel scan retained 12,747 entries, 6,373 measured files and 24.8MiB allocated contributions; terminal category pages completed. Counts depend on scheduling, not latency promises |
| Refresh/stale state | Native settings changes mark original scope stale; Refresh starts generation 2, shows explicitly stale retained generation 1 while ingesting new pages, then final excluded small-scope results |
| Investigation | Native category selection routes Media/one measured file/generation to Explore handoff; no additional scan; pending Explore inspector is explained |
| Restart/preferences safety | Rust isolated configuration roundtrip preserves scope/exclusions/appearance without any native scan session; missing root remains unavailable; invalid save leaves prior bytes unchanged; corrupt/oversized file remains untouched; 0600 mode verified |
| Error/empty/stale frontend | React tests cover empty directories, unreadable roots, cancelled/failed sessions, unavailable volume metrics, corrupt settings, persistence failure, late generation, rate limit and mutable final ancestors |
| Themes/accessibility | Native 1200×900 light and dark captures attached as SYNTHETIC evidence; AX exposes labels, native fields/radios, status and table semantics. Tab then Space opens Edit next scan without changing generation. Appearance change preserves scan freshness |

Commands actually run:

- `npm ci`: exit 0, pinned lockfile, 240 packages installed, zero audit findings.
  Initial typecheck before installation failed because `vite/client` was absent;
  installation resolved the environment prerequisite.
- `cargo test -p reclaim-core --locked`: exit 0, ten tests including literal
  exact/subtree exclusion coverage and existing no-follow/accounting fixtures.
- `cargo test -p reclaim-desktop --lib preferences --locked`: exit 0, one
  atomic configuration/corrupt-file/missing-root/native-authority test.
- `npm run check`: exit 0, formatting, ESLint, strict TypeScript, 19 frontend
  tests, workspace Clippy and 17 Rust tests. Initial focused frontend assertions
  were corrected for two status regions and documented truncating byte formatting;
  initial lint caught impure clock initialization and hook export placement.
- After the appearance-freshness correction and category-strip mapping,
  `npm test -- src/scan/session.test.tsx`: exit 0, 12 tests; `npm run lint`:
  exit 0; `npm run build:web`: exit 0, includes strict typecheck, 74 modules,
  90.80kB gzip JS. Unchanged Rust/core proof reused.
- `cargo clippy -p reclaim-desktop --example overview_fixture --locked -- -D warnings`:
  exit 0. `cargo build -p reclaim-desktop --example overview_fixture --features
  tauri/custom-protocol --locked`: exit 0. Actual native fixture runs closed with
  exit 0 and dropped their disposable directories.
- `impeccable detect --json` over changed UI sources: zero primary findings,
  three advisory findings (two existing font steps and small category swatch radius).
- `git diff --check`: exit 0.

`src-tauri/examples/overview_fixture.rs` is the repeatable native harness. It
substitutes only suggested locations and preference-file location, uses the real
native picker/scan commands, and runs the production frontend. All filesystem
measurements are from its temporary SYNTHETIC fixtures. Volume metadata describes
actual volume accounting; no real private directory was scanned. `dev_url = None`
in its generated context makes the custom-protocol harness use bundled assets.
For a fresh fixture build, run `npm run build:web`, then
`cargo run -p reclaim-desktop --example overview_fixture --features tauri/custom-protocol --locked`.
UI-tool attachment required wrapping this executable in a local ignored `.app`
with an Info.plist; this is QA tooling, not a distributable release.

Spoken VoiceOver remains unverified under the existing manager-accepted shell
environment limit (request `171285ea-b0b1-4afe-bbf3-09e69d141ff0`). Existing authored
motion and OS reduced-motion proof are reused; no new animation or OS preference
change was introduced. No older macOS, Intel, physical disconnection, APFS extent
reconciliation, release bundle, notarization or hosted CI result is claimed.

### Finish review and final evidence

The required independent Impeccable finish review found two material truth fixes:
pending destinations claimed no files had been scanned, and product capability
claims lagged delivery. Pending-view copy now says the view is unavailable and no
files are removed; `PRODUCT.md` lists implemented Overview/settings/session and
remaining feature views. The reviewer scored **both fixes resolved**, disposition
**ship at the scored-fixes scope**. This is not an unconditional whole-application
runtime or spoken-accessibility verdict. Unchanged measured-view native captures
were reused for the copy/documentation-only corrections.

The independent documenter checked both native captures, tokens, components,
existing motion, PRODUCT/spec and `DESIGN.md`/`.impeccable/design.json`: ordinary
extension preserves the incumbent system; no design files or raster assets changed.
Existing font-step and task-local swatch-radius advisories were not canonized into
a new system. After the final copy edit, `npm test -- src/shell/shell.test.tsx`
passed four tests and `npm run build:web` passed strict TypeScript and bundling.
Thirty-seven relative documentation links resolved. Local proofs do not substitute
for manager-owned hosted CI or integration.
