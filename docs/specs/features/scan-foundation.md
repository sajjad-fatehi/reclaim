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
