# Explore and shared item inspector

Follow [shared requirements](index.md), [information architecture](../../design/information-architecture.md) and the delivered scanner/shell. Reuse Overview's integrated session orchestration; do not start a separate background scan.

## Required behavior

- Browse real scan findings with breadcrumbs, scoped search, stable sorting and folder/file rows including hidden and unrecognized entries. Directory navigation and row inspection are distinct actions.
- Show measured size/basis, name, ownership/type when known and measurement status. Allow clear filter reset/back behavior, preserve selection/scroll through navigation and expose full identities for equal names in different directories.
- Support large result sets through bounded retrieval and a measured accessible rendering strategy. Deep paths, Unicode, lossless native identities and invalid-display bytes must not break navigation or escaping.
- Implement the shared inspector: path/copy, owner/evidence, size/time/basis, coverage, uncertainty, removal consequences and related-project/tool links. Unknown data stays unknown. Do not infer growth history from one scan.
- Show in Finder and Copy path are explicit user actions. The native reveal endpoint accepts an authorized finding identity, revalidates its path and uses a safe platform API without shell interpolation or an arbitrary program/URL opener.
- Reflect pending/partial/cancelled/stale/denied/vanished findings. Safe inspection remains possible when cleanup is unsupported. Add-to-review is supplied by the later cleanup task, not a fake button now.

## Acceptance and proof

- Fixture tests cover navigation, scoped searching, ascending/descending sorting, duplicate names, empty filters, long paths, partial measurements and stable selection under updates.
- Forged IDs and paths outside the scan scope cannot trigger native reveal. A disappeared target yields a useful error. No private contents or raw error dump is rendered.
- Native fixture interaction and keyboard/VoiceOver smoke evidence cover table, inspector and Finder handoff; both theme and compact/text-enlarged layout remain usable.
- Relevant focused tests and required checks pass; no destructive operation or duplicated scan-session store is introduced.

## Implementation contract (acceptance recorded before edits)

Explore consumes `useScanSession()` and `ScanSnapshot.findings`; it never creates a scan or another session store. Existing `scan_page` limits transport to 200 findings and the producer caps a scan at 100,000 entries. Explore indexes this snapshot once per update, searches descendants of the breadcrumb scope, uses deterministic ID tie breaks, and renders at most 50 rows per page. Navigation retains per-scope filters/page/scroll and ID selection; generation changes reset obsolete identities. Overview investigations retain their generation/category/IDs, begin at scan scope independently of ordinary browsing, and can be cleared to restore the prior folder.

The shared inspector consumes a `Finding`, generation and session status plus `scan_inspect` details. Native details resolve the exact ID within current native scan, return the complete escaped display path, an exact UTF-8 copy path only when representable, and nearest recognized artifact/evidence/project IDs. Display text never authorizes access. Unknown owner/history/recovery stay unknown; no review action is supplied. Related IDs open the same scoped inspector without changing scan settings.

`scan_reveal` accepts only generation and finding ID from the main window. Native authority resolves the record, checks its selected scope, walks ancestors without following symlinks, checks recorded device/inode and target metadata, then hands a file URL to macOS NSWorkspace. No shell, arbitrary opener or frontend path argument. Invalid-byte names retain native identity and file-URL support; copy is unavailable rather than lossy. Retained details remain inspectable after disappearance; reveal errors are typed and presented with recovery copy.

Proof will cover bounded rendering, navigation/search/sort, duplicates/Unicode/long paths, partial states, selection updates, forged/stale IDs and changed/missing/symlinked targets. Native fixture QA covers keyboard, accessibility tree/VoiceOver availability, Finder handoff, themes, compact/200% text and reduced motion; unavailable evidence must be stated explicitly.

## Delivered contract and verification

- `src/explore/Explore.tsx` is mounted by `ScanApplication` and consumes the existing provider. `model.ts` owns presentation indexes, descendant matching, deterministic ascending/descending ordering and 50-row pages. Name, allocated contribution, type and measurement status are shown; full native paths are available in the inspector. Selection uses finding IDs rather than display names, survives filters/updates, and resets on generation replacement. Folder-specific filters/page/scroll are retained while shell navigation keeps the view mounted.
- `src/explore/ItemInspector.tsx` exports `ItemInspectorProps` for other session consumers. Pass current finding/progress/stale/settled and callbacks; the native API is default, injectable only for fixtures. Related IDs use the same scan index and retain current breadcrumbs/filters. Close restores a connected invoker; when filtering removes that row, Explore focuses scoped search. Related selection moves focus into updated details. Native pending details ignore late responses from earlier selections.
- IPC: `scan_inspect({ generation, findingId })` returns `{ findingId, displayPath, copyPath, artifact, recognizedAtId }`; nearest recognized ancestor is explicitly qualified as inherited evidence. `scan_reveal({ generation, findingId })` is authorized to the local main window, resolves only current native records, compares the root against native granted candidate paths, and rechecks every ancestor and target identity without following symlinks. File size and modification time use the scanner's recorded granularity (seconds). Symlink handoff is inspection-only. The final Finder request is a file URL through `NSWorkspace.activateFileViewerSelectingURLs`; no process/shell/open capability exists. New direct AppKit/Foundation dependencies were already locked transitively; narrow bindings supply this native platform operation without unsafe code in the application.
- Finder handoff is non-destructive and path-based: validation occurs immediately before calling the platform API while scan authority is locked. This does not promise an atomic filesystem lock against unrelated processes renaming a path after validation. This endpoint cannot authorize cleanup.
- Invalid byte identities and complete escaped paths are tested through injected native records: APFS rejected creating an invalid-UTF-8 filename (`Illegal byte sequence`). This is transport/native-URL proof, not a claim that such a filename was created on this filesystem. UTF-8 paths copy exactly through explicit browser clipboard access; unrepresentable paths disable copy. Denied/vanished/changed/stale errors use useful fixed copy rather than native error dumps.

### Automatic evidence

- `npm run check`: passed format, ESLint, strict TypeScript, 29 frontend tests and 49 Rust tests, including added IPC wrong-window/forged/invalid-payload checks. Subsequent focused checks cover later edits below; unchanged core evidence was reused.
- `npx vitest run src/explore/explore.test.tsx`: eight focused tests cover navigation, scoped search, stable sort ties/directions, hidden/duplicate/Unicode/long paths, empty filters, partial/denied/vanished states, bounded rows, stable selection and focus, copy/reveal identity arguments, related links and late-result rejection. A synthetic 100,000-entry index/search/sort measured 137 ms locally (presentation computation only; not native/render latency); mounted table fixture stays at 50 rows plus header.
- `cargo test -p reclaim-desktop scan::explore --locked`: native fixture tests cover valid dispatch seam, forged/stale IDs, ungranted roots, changed/missing files, swapped symlink ancestors, lossless invalid-byte records, full paths beyond display truncation and inherited authoritative recognition.
- `npm run build:web`: production frontend bundle built; native fixture built with `cargo build -p reclaim-desktop --example overview_fixture --features tauri/custom-protocol --locked`. These are compile/package proofs, not manual interaction acceptance.

### Explicit environment exception

Formal Impeccable disposition: **recapture**, recorded in [Explore documentation review](../../../.impeccable/review/explore-documentation.md). No current Explore screenshots exist. Missing rendered capture matrix: standard light/dark, compact `900×600`, `200%` text, reduced motion; missing manual native keyboard/VoiceOver/Finder-handoff verification. Component semantics/focus tests and inherited reduced-motion code are not a rendered visual or native assistive-technology pass.

CUA repeatedly returned `cgWindowNotFound` for the synthetic native fixture. Narrow native telemetry independently reported a visible `1440×988` window at `(240, 30)` and native window ID `17916`; CUA rejects direct macOS window-ID attachment. This telemetry does not prove interaction acceptance. Browser inventory was empty; Chrome tab creation returned `Browser is not available: chrome`, and native Chrome attachment failed with the same window error. No refusal was bypassed and attachment retries stopped.

The manager explicitly accepted native evidence limits in request `d183494e-46a6-4443-8061-8af71b47864c`, then independently confirmed empty browser inventory and accepted browser capture limits in request `94a99f2e-5e5e-49f5-9fc5-4ec5c4fc75ba`. Issue `7c812ff8-f93a-497a-8de2-98f995b17f7b` records this unresolved environment finding; acceptance of the limit does not assert it was fixed.

For future supported QA: `npm run dev:web`, then `/?fixture=explore` mounts a clearly labelled development-only synthetic provider and production components. It never invokes native scanning or Finder. Native `overview_fixture` adds hidden/Unicode/duplicate/paginated/project fixtures and supports `--compact` / `--enlarged`; measurements require an explicit Start scan action. No cleanup operation was implemented or tested.
