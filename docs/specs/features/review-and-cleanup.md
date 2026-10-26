# Review, limited cleanup and operation history

Follow [shared requirements](index.md), [interaction flows](../../design/interaction-flows.md), the delivered scanner/artifact contracts and all completed views. This is the only feature authorized to introduce mutation; verification still never touches real user data.

## Eligibility policy

- Source, user documents, credentials, arbitrary logs/tmp content, editor history/chats/settings, persistent volumes, running resources, main repositories and unknown artifacts are not generic cleanup targets.
- Support a small explicit allowlist of verified regenerable project outputs/dependencies and tool caches with an accurately described removal method. Prefer recoverable platform Trash for eligible filesystem items where supported; disclose when recovery is not supported. Never silently fall back from failed Trash to permanent deletion.
- Shared stores/runtimes need evidence of tool-supported scope and consequences. Do not run broad autoremove/prune/uninstall, remove installed toolchains or claim that cache deletion is risk-free. Unsupported stores remain inspection-only with a clear reason.
- A linked Git worktree may be removed only through Git's supported non-force mechanism after validating registry membership, non-main status, absence of tracked/untracked/ignored data, locks and concurrent changes. Otherwise block and explain; never recursively delete a worktree directory as a substitute.
- Docker cleanup is limited to explicitly selected, supported unreferenced image identities after fresh local-daemon/context/reference checks, with no force flag. No volumes, containers or broad prune. If an installed version cannot guarantee this scope, mark the method unsupported. Persistent/build-cache inventory remains useful without blanket removal.

## Native execution boundary

- Review plans are native-owned, bounded, short-lived and tied to exact finding identity, session, scope, measurement time, method and consequences. Frontend submits opaque IDs, never executable commands, trusted sizes or arbitrary paths.
- Revalidate targets immediately before mutation. Reject traversal, symlinks/ancestor substitution, out-of-scope aliases, identity changes and new safety-relevant changes. Use filesystem/platform primitives that enforce the boundary under concurrent replacement; a prior path check alone is insufficient. Fail closed if the intended method cannot preserve this invariant.
- Prevent duplicate/concurrent execution of the same target. Handle parent/child overlap without double-removal; user confirmation for an old plan never authorizes a changed replacement.
- Execute only after a separate explicit confirmation. Cancellation stops pending work safely; completed work is not described as undone. Report Removed, Skipped, Failed or Changed per item and bounded useful errors. Never hide partial failures or retry by widening scope.

## UI and history

- Wire actual Add to review actions from eligible Explore/Development/Overview contexts to shared selection. Navigation/filtering must not silently expand it; stale/unsupported items cannot masquerade as eligible.
- Review shows exact target, measured size/basis/time, owner/evidence, method, removal consequences, regeneration costs and reversibility. Final confirmation is specific and Cancel-focused; Enter does not accidentally approve permanent deletion.
- During execution show truthful progress and prevent duplicate submission. Provide scoped retry only after new eligibility checks. Supported recovery appears only when actually available.
- Show measured volume free-space change separately from selected apparent sizes, qualifying concurrent activity and APFS. Unknown recoverable amounts remain unknown.
- Persist a bounded local operation history inside Review & Clean using a minimal appropriate store with validated atomic writes. Include time, method and per-item outcomes; avoid unnecessary sensitive data. Offer clear-history, retain actual unfinished/unknown outcomes after interruption and never automatically replay pending deletions on restart.

## Acceptance and proof

- Disposable-fixture tests cover allowlisted success and negative cases: protected source/credentials, forged IDs, outside-scope paths, symlink/ancestor replacement, changed targets, overlapping selections, double submission, partial failure, cancellation, corrupt/interrupted history and safe recovery failure. Verify outside-scope sentinels survive.
- Worktree tests include ignored/untracked data and no-force behavior. Docker deletion uses stubbed commands, validates local identity/reference scope and cannot issue volume/container/broad-prune operations.
- Native fixture UI flow proves selection → review → explicit confirmation → actual permitted fixture operation → truthful outcomes/history across restart. No scanning or deleting the user's private files to make screenshots.
- Focused Rust/transport/UI tests, security-boundary checks and required CI pass. Report any narrowly unsupported method honestly; do not replace promised safe behavior with a generic recursive delete.
