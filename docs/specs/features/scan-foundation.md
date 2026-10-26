# Read-only scan foundation

Follow [shared requirements](index.md). This producer establishes the contract later features consume; no existing scanner is assumed.

## Outcome and ownership

Provide real, cancellable, bounded-resource read-only filesystem measurement through a Tauri-independent Rust core and a narrow desktop adapter. Publish the actual typed scan/scope/identity/error contract in this specification before delivering it. No full UI redesign or cleanup belongs here.

## Required behavior

- Expose readable volume metadata and candidate user-owned locations, with explicit metric/time/permission semantics. Suggestions are not permission to scan them.
- Provide a native folder-selection route and validated scope registration. Start only for an explicitly submitted, validated scope; never implicitly scan all of HOME or the volume. Reject malformed/unapproved identifiers and unreasonable requests at the native boundary.
- Assign session/generation and finding identities. Support start, bounded progress/result retrieval, current status and cancellation with only the commands required. Prevent conflicting scans or stale updates from replacing a newer session. No unbounded stream or collection returned in one IPC response.
- Preserve completed findings on cancellation or partial failure, with coverage state. A denied directory, vanished file or unreadable entry must not become zero bytes or a complete scan.
- Traverse metadata without reading file contents. Handle hidden files, non-UTF-8 paths, symlinks, overlaps, hard links, cycles, package directories and files changing during traversal. Keep authoritative lossless paths native; provide safe display strings and opaque identifiers to the UI.
- Account once within a scope; explicitly distinguish volume totals, scan scope and unexplained/unscanned usage. Expose allocated/logical size and their limitations without promising APFS clone/snapshot reclaim estimates.
- Bound work concurrency, event rates and result retrieval. Keep the UI thread responsive; cancel cooperatively without waiting for the full tree. Do not cross filesystem/mount boundaries silently.
- Add only narrow capabilities required by this behavior. Path-reveal or cleanup commands are outside this task. Permission denial is an honest result, not grounds for privilege escalation.

## Acceptance and proof

- Fixture scans produce correct hierarchical aggregates without double-counting overlapping roots or hard links; symlinks/cycles cannot escape scope.
- Invalid scopes, unavailable metrics, denied/vanished entries, large/deep trees and cancellation produce typed predictable results; a stale generation cannot replace a new one.
- A native smoke check demonstrates metadata-only traversal of disposable fixtures and real IPC progress/cancellation; no unrestricted filesystem/shell frontend plugin is exposed.
- Focused Rust/transport tests and required repository checks pass. Record the actual public contract, resource bounds and measured fixture behavior, not speculative performance claims.
- Update relevant setup/security documentation for actual new permissions. Preserve data and existing metadata behavior; do not implement cleanup.

## Producer design and acceptance record

Implementation owns `crates/reclaim-core/src/scan.rs` (domain DTOs, anchored traversal,
accounting) and `src-tauri/src/scan.rs` (native grants, session lifecycle, commands).
No frontend contract or presentation changes are required by this producer.

Before implementation, the selected invariants are:

- A native picker or a native candidate identifier is explicitly registered before
  starting. No command accepts a frontend filesystem path. Grants and sessions are
  process-local; saved display paths cannot restore authority.
- Open roots and child directories using descriptor-relative, no-follow operations.
  Retain native lossless paths and device/inode identity; UI receives bounded lossy
  display strings and session-local finding IDs. Root replacement cannot redirect
  an already-open grant. Descendants are opened relative to their parent handle.
- Serial worker, at most 16 roots, depth 128, 100,000 retained entries, 200 entries
  per response, 1,024 display characters per string. Stop with partial coverage on
  resource limits. No per-file events; clients poll, with native throttling.
- Logical/allocated measurements count each device/inode once across all roots.
  A hard-link alias contributes zero additional bytes and points to its first finding.
  Directory totals aggregate child contributions, not filesystem directory blocks.
  APFS clone sharing and snapshots remain unknown; these are not reclaim estimates.
- No symlink traversal; hidden files and packages are ordinary entries. Mounts are
  visible boundaries and are not descended. Changed, unreadable and vanished entries
  retain explicit issue states. Completed entries survive cancellation and failures.
- One active session; stale generation requests fail, and running work cannot be
  replaced. New sessions replace only terminal data. Cancellation is cooperative
  between filesystem operations; a blocked OS call has no hard latency promise.

Focused fixtures will verify hierarchy, overlap, hard links, sparse files, hidden
and package entries, symlink escape/cycles, non-UTF-8 names, resource limits,
cancellation, invalid authority and pagination, stale generations, mount decisions,
and deterministic denied/vanished/change handling. Required gates are `npm run
check`, workspace Rust tests, and native fixture IPC smoke. Runtime evidence and
exact transport details will be added below after implementation.

## Delivered transport contract

Rust DTO definitions are authoritative: `reclaim_core::scan::{Finding, Progress,
Volume, VolumeAccounting, Issue, Kind, Status}` and
`reclaim_desktop::scan::{Location, Locations, Scope, Current, Page, BoundaryError}`.
DTOs serialize with camelCase keys and snake_case enum values. All byte quantities
are decimal **strings** (nullable where unavailable); timestamps are Unix
milliseconds, generations/counts/offsets are bounded JSON numbers. No TypeScript
casts or duplicate frontend domain implementation are supplied.

Invoke these app commands from the local `main` WebView using Tauri `invoke`:

| Command | JSON arguments | Success |
| --- | --- | --- |
| `scan_locations` | `{}` | `{ candidates: Location[], unavailableCount }`; lazily lists up to five readable, current-user-owned suggestions plus already picked folders; no traversal |
| `scan_pick_folder` | `{}` | Native directory picker; `Location` on selection, `null` on cancellation; at most one picker open |
| `scan_register_scope` | `{ locationIds: string[] }` | `Scope { id, roots: Location[] }`; 1–16 existing native candidate IDs, duplicates removed; explicit user submission authorizes these roots |
| `scan_start` | `{ scopeId: string }` | Initial `Progress` for a new generation; starts one background worker; rejects stale/replaced root paths |
| `scan_current` | `{}` | `{ scope: Scope \| null, progress: Progress \| null }`; starts no work |
| `scan_status` | `{ generation: number }` | Current `Progress` |
| `scan_page` | `{ generation: number, offset: number, limit: number }` | `Page { progress, offset, nextOffset: number \| null, findings: Finding[] }`; limit 1–200 |
| `scan_cancel` | `{ generation: number }` | Current `Progress`; requests cooperative cancellation, never claims synchronous completion |

`Location` contains random UUID `id`, bounded `displayPath` and `volume` metadata.
`Scope` is a random UUID plus selected locations, retained only in native memory.
Picking/suggesting a folder does not register a scope or start a scan. A native
candidate can be an ancestor of another candidate; the core normalizes overlaps on the same mount
before traversal; an explicitly selected nested mount remains a separate root. `Scope.roots` records the requested selection; findings contain
only effective roots. No command accepts a frontend path, findings or size claim.

One scope and one scan session are retained. Native monotonic `generation` starts
at 1 and increments only on accepted starts; scope/candidate UUIDs and generations
are process-local, with no persistence or authority restoration from display paths.
A running scan prevents start/scope replacement. New terminal sessions replace old
findings; requests using an old generation return `stale_generation`. Worker state
is separately owned per generation, so old work cannot overwrite replacement data.

`Progress` contains generation, status, startedAtMs, finishedAtMs, entries,
issueCount, terminalIssue, logicalBytes, allocatedBytes and volumes. Status is
`running`, `complete`, `partial`, `cancelled` or `failed`. `finishedAtMs` remains
null while running. Counts describe measured entries, never a guessed percent or
an estimated total. `issueCount` counts observations, so one entry can contribute
more than one issue; `Finding.issue` retains its latest issue.

`Finding` contains opaque `id` (`generation:index`, not a path authorization),
parentId, displayName, kind (`directory`, `file`, `symlink`, `other`, `unknown`),
logicalBytes, allocatedBytes, measuredAtMs, nullable modifiedAtMs, nullable identity
(device/inode observation), aliasOf, complete and nullable issue. Root displayName
is its bounded display path; descendants contain only their display name. Native
`ScanData::path(index)` reconstructs authoritative lossless paths from retained
parent/name components. `ScanService::with_terminal_scan` provides native consumers
short read-only access to terminal core records; it rejects running/stale data.
Consumers must copy needed metadata before expensive analysis. Identity reuse and
subsequent filesystem changes require fresh validation before any future cleanup;
these observations never establish deletion eligibility.

Records append during traversal; aggregates and completeness change in place.
Pages are not immutable snapshots while running. Poll progress for live status,
then retrieve/retrieve-again terminal pages for final directory aggregates. A null
nextOffset means end of **currently retained** records, not completion; check status.
Empty authorized directories still have a root finding. Completed file findings
and measured bytes survive cancellation/errors. Interrupted ancestor coverage is
incomplete; unreadable/vanished entries have explicit issues and unknown measurement,
not evidence of an empty directory. Their numeric contribution is zero **only as
an accounting contribution**; `complete: false`/issue must remain visible.

`scan_current`, `scan_status` and `scan_page` share a per-session 100 ms read budget
(maximum 10 successful retrievals/second). Call one at a time; retry `rate_limited`
after at least 100 ms. No events, timers or unbounded result streams are emitted.
Cancel/start requests do not consume that read budget.

## Errors, safety and accounting

Commands reject with `{ code, issue }` (issue nullable). Codes: `invalid_request`
(empty/oversized selection, bad page), `invalid_scope` (unknown/stale grant),
`stale_generation`, `busy`, `rate_limited`, `access_denied` (window/origin),
`unavailable` (native filesystem failure) and `internal` (worker startup/runtime
failure). Tauri deserialization and ACL rejection precede command execution and
can return framework error strings; consumers must handle those as transport
failures rather than assuming every rejection is a domain DTO. Errors/logs do not
include private paths, file contents or raw OS error text.

Issues: `permission_denied`, `vanished`, `io`, `changed`, `mount_boundary`,
`depth_limit`, `entry_limit`, `cancelled`, `worker_failed`. Recoverable entry issues
produce terminal `partial`; cancellation produces `cancelled`; a worker panic
retains observations and produces `failed`. Invalid requests leave the session
unchanged. A failed start caused by an unavailable/replaced grant requires selecting
a valid native folder again; no privilege escalation or permission bypass occurs.

Traversal opens directory components relative to anchored handles with
`O_DIRECTORY | O_NOFOLLOW`; no regular-file content is opened. It never follows
symlinks, including ancestor symlinks at core grant opening. The native picker and
suggestions canonicalize their chosen location first. Directory identities suppress
cycles/aliases; device changes and macOS mount-target changes stop descent with
explicit coverage issues. Hidden files and package directories are traversed
normally. Native names remain lossless; display strings replace invalid UTF-8,
control characters and bidi override/isolation controls, then cap display length.
APFS rejects invalid UTF-8 filenames in this verified environment; preservation is
therefore tested with synthetic native records rather than a falsely claimed
on-disk APFS filename fixture.

Each observed regular-file device/inode contributes logical `st_size` and allocated
`st_blocks * 512` once across selected roots, with subsequent aliases contributing
zero additional bytes. Directory totals sum attributed descendants; filesystem
directory metadata blocks, symlink storage and special-file storage are excluded.
An alias reference identifies the first observation; enumeration order determines
attribution among siblings, without changing scope totals. The scanner does not
identify APFS clone extents or infer inactivity from modification time.

Volume metadata uses descriptor-based `fstatvfs`, with fresh timestamps per scan:
totalBytes, freeBytes (all free blocks), availableBytes (available to current user),
usedBytes (total minus free), metricIssue and device id. Unsupported/failed metrics
remain null. `Progress.volumes[]` contains volume metadata, scopeLogicalBytes,
scopeAllocatedBytes and unscannedOrUnexplainedBytes. The latter is a checked
arithmetic difference between volume used bytes and measured allocated contributions;
null if metrics are unavailable or the subtraction is inconsistent. It includes
unscanned files, filesystem overhead, snapshots and unrelated usage, and is **not**
physical ownership or reclaimable space. APFS clone sharing can make allocated sums
exceed physical used space. Volume snapshots are not transactionally synchronized
with traversal. Candidate `Location.volume` reflects selection time; use progress
volume measurements for the scan.

Before descending, compare no-follow metadata against the opened directory identity
and change stamps. Recheck file metadata and each directory after enumeration;
detected changes produce partial coverage. Open handles prevent replacement paths
and symlinks from redirecting traversal. A rename of an already-authorized open
object does not revoke that object grant. This is a measurement, not an atomic
filesystem snapshot: changes after a recheck, edits with indistinguishable metadata,
or movement during a scan cannot be completely detected. No cleanup safety claim
is made from scan results.

## Resource bounds

- One scan worker, serial traversal; no per-file tasks and no file-content buffers.
- 32 retained native candidate handles, 16 selected roots, 100,000 retained findings,
  depth 128, root paths up to 4,096 native bytes; one retained terminal session.
- Directory traversal retains at most 128 directory handles plus 128 directory
  iterators, in addition to candidates/root clones and short-lived validation handles.
  Limits can still meet a lower OS descriptor limit; that becomes explicit partial I/O
  coverage, not silent success.
- Findings retain parent indexes and one native name component, not full copied
  descendant paths. Identity index is bounded by retained findings. Display strings
  contain at most 1,024 Unicode scalar values; page count is capped at 200.
  Result memory scales with retained findings, never unlimited tree size. Byte-level
  RSS/latency SLOs are not claimed; allocator/OS overhead varies.
- Entry/depth limits produce explicit partial results; no hidden continued traversal.
  Status reads copy constant-size progress plus at most 16 volume summaries. Page
  reads copy at most 200 DTOs. Oversized IPC envelopes may still be deserialized by
  Tauri before command validation; there is no custom unrestricted message channel.
- Cancellation is checked before each entry/root and between directory operations.
  It preserves measured data; no bounded response time is promised for a blocked
  filesystem syscall (for example a disconnected network mount).

Dependencies are limited to Serde for DTO serialization, Rustix for safe
no-follow/descriptor filesystem and ownership syscalls, UUID for native grants,
and Tauri's native dialog adapter. The dialog adapter transitively includes its
filesystem plugin, but no filesystem or frontend dialog ACL permission is granted.
Tempfile/serde_json/Tauri test support are development-only proof dependencies.

## Verification evidence (October 27, 2026)

Verified on Apple Silicon macOS 27.0.1 with repository-pinned Rust 1.90.0 and
Node/npm setup. Evidence covers this producer, not future consumer UI or cleanup.

| Acceptance | Actual proof |
| --- | --- |
| Hierarchy, overlap, hard links, packages/hidden files, sparse allocation | Core fixture tests; 1,000,008 logical bytes counted once with sparse allocated size lower than logical size |
| Multiple scopes on one volume | Disjoint-root fixture with a cross-root hard link counts seven logical bytes once, one volume summary and matching checked unexplained remainder |
| Symlink escape/cycles and root replacement | Out-of-scope fixture sentinel remains unchanged; no symlink descent; open root stays on original object and native replaced-grant starts are rejected |
| Permission/vanished/change/mount decisions | Real denied directory produces partial root coverage; vanished entry and pre-change directory metadata produce typed issues; mount/device branch uses a simulated differing device, without mounting or privileges |
| Resource/cancel coverage | A 500-file fixture stops at an injected 20-record limit; a nested fixture stops at injected depth three; same implementation uses production caps of 100,000/128; no giant-tree RSS benchmark claimed |
| Lossless names and unknown volume metrics | Synthetic non-UTF-8 record reconstructs native name; display controls/truncation tested; injected metric denial returns null measurements |
| Native transport | Tauri dispatcher uses generated production ACL, checks malformed/unknown scopes, byte-string serialization, wrong-window denial, paging, rate limits, busy/stale requests and worker failure |
| Real native IPC | `cargo run -p reclaim-desktop --example scan_smoke --features tauri/custom-protocol --locked` exits 0 and reports `PASS: smallBytes=5&liveEntries=51&cancelledEntries=109&page=109&stale=rejected` |

The real smoke uses only its own five-byte file and 20,000-file synthetic fixture.
Its native WebView observes live progress, requests cancellation, reads retained
findings and rejects the old generation. Fixture counts vary with scheduling; they
are observations, not latency targets. The labelled screenshot attached to the task
records another run (59 live entries, 125 cancelled/retained entries). The harness
uses `run_return` and drops its temporary fixture before exiting.

Core tests cover nine cases; native authority/IPC tests cover four. Native
folder-picker interaction is implemented through the official Rust dialog adapter
but was not manually driven during this producer verification. No real mount,
non-macOS, Intel, older macOS, APFS extent sharing or hosted CI result is claimed.

Gate commands and outcomes:

- `npm run check`: exit 0, including two frontend and twelve Rust tests at that
  checkpoint, strict TypeScript, formatting, ESLint and workspace Clippy.
- After the selected-mount overlap correction, `cargo test -p reclaim-core --locked`:
  nine tests passed; `cargo clippy -p reclaim-core --all-targets --locked -- -D warnings`:
  passed. Existing four native transport tests remain unchanged. The new normalization
  regression uses synthetic fixture paths with read-only `/dev` mount metadata;
  it never enumerates `/dev` or performs a mount operation.
- Native smoke command above: exit 0. Its single-root IPC behavior is unchanged by
  the overlap correction; no redundant native smoke run was needed for that edit.
- `npm run build`: first attempt failed with `No space left on device (os error 28)`.
  Removed only this worktree's generated Rust incremental artifacts; retry outcome
  is recorded below. No user data or other worktree was removed.
- Relative documentation links: 31 resolved; `git diff --check`: passed.

Release retry: `npm run build` exited 0, producing the ad-hoc-signed
`target/release/bundle/macos/Reclaim.app` (9.18 MiB). No notarization or hosted CI
is claimed. The only release-build failure was local disk exhaustion, resolved
without changing dependencies or touching other worktrees/user data.

Byte quantities use unsigned 64-bit storage and saturating arithmetic. Values at
`18446744073709551615` are capped, not proof of an exact apparent-size total beyond
that representable range; consumers must qualify capped values. Coverage status
describes enumeration/entry errors, independently of this numeric representation
limit. Ordinary fixture sizes and per-volume arithmetic are verified above.

## Explicit reselection after root replacement

Acceptance: selecting the same canonical path reuses its location ID only when
its opened device/inode identity still matches. Explicitly selecting a replacement
object issues a new location ID, retires the old candidate and invalidates any
registered scope containing it. A new scope registration is required; start never
refreshes or retargets authority. Stale location/scope IDs fail closed. Reselection
can replace a cached candidate even at the 32-candidate limit. Existing scan-session
observations and their anchored worker handles remain unchanged.

Isolated regression: grant/register a folder, replace it, reject the old start as
changed, explicitly reselect the same path, reject stale identifiers, register a
new scope and scan only the replacement. Repeated selection of the unchanged
replacement retains its new location ID.


Repair evidence:

- Before repair, `cargo test -p reclaim-desktop --lib explicit_reselection_replaces_stale_grants_and_requires_new_scope --locked`
  failed because explicit reselection returned the original location ID.
- After repair, `cargo test -p reclaim-desktop --lib --locked -- --skip ipc_dispatch_serializes_bytes_rejects_invalid_payload_and_wrong_window`
  exited 0: five focused authority tests passed, one unchanged IPC test excluded.
  Regression scans only the 11-byte replacement and preserves the moved original;
  retired location/scope IDs fail before and after new registration. At capacity,
  replacement grants still succeed while additional locations remain rejected.
- `cargo fmt -p reclaim-desktop -- --check` passed.
- `cargo clippy -p reclaim-desktop --lib --tests --locked -- -D warnings` passed.
- Existing release-build and native WebView IPC evidence above is reused; neither
  was rerun for this native authority repair.

## Overview consumer extension

The [Overview contract](overview-and-scope.md#native-configuration-and-exclusion-extension)
adds bounded native preferences, reviewable saved candidates, frozen literal scope
exclusions, `Issue::Excluded` and `Progress.currentLocation`. Existing scan command
callers remain compatible; omitted exclusions mean none. This extends the same
scanner and native scope registry, not a second producer. Persisted paths cannot
start scans; explicit registration and native identity validation remain required.

## Recognition consumer extension

The development-recognition producer now runs bounded recognition after the
metadata-only walker and before this worker publishes terminal status. Existing
measurement DTOs, scope, generation, page and cancellation semantics remain intact.
`ScanData.recognition` and `recognition_page` share this native session; see the
[recognition contract](development-recognition.md) for selective metadata-content
reads, optional confined Git status, limits and inspection-only eligibility.
`scan_cancel` cancels either phase. Measurement-only descriptions above concern
the walker, not the recognition phase's explicitly documented content reads.
