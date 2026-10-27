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

## Implementation acceptance record (2026-10-27)

This task preserves all requirements above. Recognition and Finder handoff are not
mutation authority. Reuse `ScanService::with_terminal_scan`, native finding identities,
recognition evidence, shared scan presentation, shell dialogs and semantic tokens.

- Selection is explicit and bounded; session changes invalidate confirmation.
- Native plans expire, are single-use and contain native-resolved exact targets.
- A method is eligible only when its actual mutation primitive enforces expected
  target identity and scope under concurrent substitution. Path validation followed
  by pathname Trash/Git execution does not satisfy this requirement. No deletion
  fallback is permitted. Method support remains under investigation; an unsupported
  method must carry a specific explanation, never fabricated successful execution.
- Persist at most 50 operations with at most 100 item outcomes each. History is local,
  private, validated and atomically replaced. Store method, timestamp, bounded target
  label and outcome; no contents, credentials, executable commands or replay tokens.
  Interrupted pending outcomes become unknown and never execute on startup.
- History corruption is visible; no fabricated empty-success history. Clearing
  history removes historical records only, never targets or pending execution.
- Fixture safety proof must cover protected/unknown targets, forged/stale IDs,
  changed targets/ancestors, overlap, concurrency, partial/cancelled operations,
  unsupported recovery and interrupted/corrupt history. Native UI evidence must
  distinguish permitted actual operations from unsupported/blocked methods.

Open security question: existing path/URL Trash and Git interfaces do not expose an
expected-identity condition. SDK rename flags prevent symlink traversal/overwrite but
have no expected-inode argument. No filesystem/worktree mutation implementation is
accepted until a scoped primitive is demonstrated or the manager resolves this
contract conflict. This record does not waive the required eligible fixture flow.

## Partial delivery and native contract

Manager disposition (2026-10-27): filesystem Trash and Git worktree removal remain
unsupported. Their mandatory successful filesystem fixture removal is replaced by
explicit fail-closed fixture results and sentinel survival. Unproven Docker methods
also remain unavailable. **All methods unavailable does not complete cleanup
acceptance.** Independent eligibility, review and history are delivered for review;
no actual cleanup success is claimed.

- `cleanup_review({ selections })` accepts 1–100 native finding references only:
  `{ source: "filesystem", generation, findingId }` or
  `{ source: "docker", findingId }`. Extra fields are rejected. Duplicate exact
  references collapse. Frontend paths, commands, measurements and force flags grant
  no authority. A forged, stale or unobserved reference rejects the request.
- Filesystem references resolve through the existing terminal `ScanService` data,
  exact recognition and authorized roots. Recognition of an ancestor does not make
  a descendant eligible. Existing descriptor-based read-only validation detects
  known target/ancestor replacement; it is explicitly not mutation authorization.
- Docker references resolve only against the latest native inventory cache, with
  native observation time. Failed inventory clears that cache. Review starts no
  Docker processes and sends no HTTP requests. Unreferenced images still receive
  an unavailable-method explanation; containers/volumes remain protected.
- Reports contain exact native display target, logical/allocated observations,
  measurement/check time, ownership/evidence/consequences and unavailable/changed
  explanation. A report is not an executable plan. Parent/child selections remain
  explicit; no overlapping sum or reclaimed-space total is computed.
- Overview lists exact scanned roots for review; Explore and Development share
  `ItemInspector` actions. `Review cleanup options` queues an eligibility inquiry,
  with a persistent selection shortcut. Unsupported findings never receive an
  eligible `Add to review` or confirmation/removal/recovery control. Navigation and
  filters preserve selection; stale filesystem generations require fresh selection.
- `cleanup_history()` reads only `cleanup-history.json` under native app config;
  `cleanup_clear_history()` atomically replaces it with empty history. Only the
  local main window has these permissions. No filesystem cleanup or shell
  capability, execution, retry or recovery endpoint is exposed.
- Domain history validates at most 50 operations × 100 outcomes, bounded labels
  and explanations, methods/outcomes, unique operation IDs and timestamps. Storage
  additionally caps serialized history at 4 MiB. Descriptor-relative exclusive
  temporary creation and rename avoid following a replaced final history symlink.
  Writes use mode `0600`, file sync, atomic rename and parent-directory sync. Reads
  reject symlink/nonregular/hard-linked, wrong-owner/nonprivate, oversized or invalid
  files. An orphan temporary file is never accepted as completed history.
- Loading converts pending outcomes to unknown without replay; completed outcomes
  stay intact. Corrupt/inaccessible history is visible and untouched until an
  explicit clear. This build produces no removal operations; production history is
  empty unless existing valid records are present. Synthetic interrupted records
  occur only in the explicitly labelled native fixture.

### Missing mutation guarantees

macOS SDK `NSFileManager::trashItemAtURL` exposes a URL operation, without an
expected-inode condition. `renameatx_np` no-follow/beneath/exclusive flags do not
provide that condition. Quarantining a substituted target before checking identity
would itself move an unauthorized item. Git's pathname removal has no demonstrated
atomic registry/data safety guarantee here. Neither method is delivered.

Examined [Moby v28.0.0 `daemon/images/image_delete.go`](https://github.com/moby/moby/blob/v28.0.0/daemon/images/image_delete.go):
`ImageDelete` and `imageDeleteHelper` call `checkImageDeleteConflict`, then separately
remove references and call `imageStore.Delete`. No guard spanning those observations
and mutation was demonstrated. The current adapter's `LocalTransport::get` launches
separate curl connections; `collect` does not pin daemon identity across requests.
This is **missing proof**, not a reproduced exploit or a conclusion that every Docker
version is unsafe. A future supported method needs exact immutable image identity,
fresh supported local context, stable daemon identity and version-specific reference
guards at mutation, without force or parent pruning, plus stubbed race/scope proof.

Executable short-lived plans, final removal confirmation, execution progress,
per-item mutation results, cancellation/partial-failure/revalidated retry, actual
recovery and permitted-operation fixture flow remain unmet. Never substitute a fake
successful operation for these requirements.

### Verification of partial delivery

- `cargo test -p reclaim-core cleanup --locked`: bounded history validation and
  pending-to-unknown normalization, preserving completed outcomes.
- `cargo test -p reclaim-desktop cleanup --locked`: native reference/transport
  rejection, source/secret/outside sentinel survival, duplicate/overlap handling,
  target/symlink/ancestor substitution, worktree safety-state policy, native Docker
  observation-only review and private atomic/corrupt/interrupted history behavior.
  Worktree safety states are synthetic policy fixtures, not successful live Git removal.
- `rtk vitest run src/cleanup/cleanup.test.tsx`: explicit selection across navigation,
  fail-closed native explanations, suppressed duplicate checks, mismatched responses,
  corrupt-history clearing and malformed transport rejection.
- `npm run build:web` and
  `cargo build -p reclaim-desktop --example cleanup_fixture --features tauri/custom-protocol --locked`:
  production React and native fixture compile. Fixture roots/config/history are
  disposable and labelled SYNTHETIC; no live Docker transport or actual removal.
- Native capture attempt: CUA `getApp` returned `cgWindowNotFound` for both the
  fixture `.app` wrapper and bundle ID. Inventory confirmed fixture running; no
  supported browser surface was listed. Native interaction, screenshot, light/dark,
  compact/enlarged-text and VoiceOver speech remain unverified. Compilation and
  React/IPC tests do not establish those visual results.
- Impeccable detector: only three pre-existing off-ramp advisories in unchanged
  portions of `src/styles.css`; no new deterministic findings. Formal visual finish
  review is unavailable without valid captures. Existing design sidecar drift was
  observed but not changed as part of this task.

- `rtk npm run check`: passed formatting, ESLint, TypeScript, 47 frontend tests,
  workspace Clippy (`--all-targets --locked -- -D warnings`) and 59 Rust tests.
- `rtk npm run build`: passed; produced ad-hoc signed
  `target/release/bundle/macos/Reclaim.app`. This is a local build, not notarized
  distribution or a tested native interaction.
- Changed documentation link targets and `git diff --check`: passed.

Hosted CI belongs to the manager's PR/integration flow and is not claimed locally.
