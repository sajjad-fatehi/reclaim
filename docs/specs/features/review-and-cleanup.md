# Review, limited cleanup and operation history

Status: implementation contract updated 2026-10-27 under current user decision
(note `5b88330e-9534-48f8-b606-be0a1628faf6`). This section supersedes the
historical absolute concurrent-substitution gate and review-only disposition below.
Follow [shared requirements](index.md) and [workflow](../../development/workflow.md).

## Current authorization and safety model

Explicitly selected allowlisted targets, exact review and separate confirmation,
immediate native scope/identity/change revalidation, then normal macOS Trash are
approved. Descriptor/no-follow checks reject observed substitutions; the subsequent
pathname operation is not atomic with those checks. Concurrent changes in that
interval remain a residual risk. No guarantee of zero races is made. No permanent
recursive-delete fallback, Trash emptying, force, broad prune or automatic cleanup.

## Eligibility and plan contract

- Native allowlist: verified project `node_modules` with installer evidence and
  Cargo `target` with `cargo_rustc_fingerprint` evidence. Other families remain
  inspection-only until their specific method is implemented. These directories
  may contain locally edited generated files; confirmation explains that these
  move too and regeneration may require network/toolchain/inputs.
- Exact recognized directory only, complete current measurement, no alias, root
  target, protected descendant, nested repository/worktree, excluded/unreadable
  subtree, symlink escape or unknown identity. Source outside selected generated
  targets, credentials, personal/editor/persistent data, logs/tmp and runtimes stay
  protected. Recognition alone never authorizes a mutation.
- Native plans bind opaque IDs, generation, scope, full target/ancestor and evidence
  observations, method and consequences. Maximum 100 targets, one current plan,
  120-second expiry, consumed once. Parent/child overlap is skipped without expanding
  selection. UI paths/sizes/commands are never trusted input.
- Immediately before dispatch, verify current generation/scope, the target subtree
  fingerprint and evidence/ancestor observations, current recognition/protection and
  exclusions. Known changes require fresh scan/review. Checks are bounded by the
  scanner depth limit, not by the retained-detail budget; targets with more than
  250,000 entries qualify. No contents or paths are logged.
- Target subtree fingerprint (decision note `0f406613-d5c9-4d3b-99bd-390bb2cf8ffb`):
  a fresh, unpruned descriptor-anchored walk of the target, independent of what the
  scan retained, computed at review/eligibility time and again immediately before
  dispatch. It covers entry count, logical bytes, allocated bytes and an
  order-independent SHA-256 multiset digest over every entry's relative name bytes,
  kind, dev:ino, size and mtime (nanoseconds). Plans carry the fingerprint; dispatch
  proceeds only if a fresh walk is equal. At review the count and byte totals must
  also equal the scan's measurement of the target, and the root identity and mtime
  must equal the scan record. Any walk issue inside the target (unreadable, vanished,
  changed during the walk, mount boundary, depth limit, interior hard-link alias,
  unknown type, or a `.git` entry under a Trash target) makes the target ineligible.
  Add, remove, rename, inode replacement, size or mtime change anywhere inside the
  target, retained or not, changes the fingerprint. Equality is hash-based; a
  per-entry difference report is not produced.

## Methods

- Filesystem: `NSFileManager.trashItemAtURL`; success means moved to Trash, not freed
  physical space. Recovery uses macOS Trash / Put Back where available; no app-driven
  restore promise. Failure never falls back to permanent removal. Ambiguous effects
  are Unknown and require inspection before a fresh review.
- Git: only registry-verified, non-main, unlocked linked worktrees with fresh clean
  tracked/untracked/ignored checks, supported local configuration, using official
  non-force `git worktree remove`. No recursive-delete substitute. Nonrecoverable.
- Docker: only native-observed immutable unreferenced images, fresh supported local
  context/endpoint/daemon and container/reference observations; fixed exact-ID API
  removal with `force=false&noprune=true`, no automatic retry or endpoint fallback.
  Containers, volumes and broad build-cache removal remain prohibited. Fresh checks
  are ordinary checks, not cross-client atomic transactions. Conflict/errors may
  follow partial untagging; ambiguous responses stay Unknown. Tool-native methods
  require their own explicit nonrecoverable confirmation, separate from Trash.

## Execution, UI and history

Eligible items offer Add to review; unsupported selections show reasons. Selection
stays explicit across views/filters; no default broad selection. Review displays
exact target, measurements/basis/time, evidence/owner, method, consequence and
recovery. Confirmation names exact executable targets, starts on Cancel and requires
an additional explicit acknowledgement for irreversible methods. Enter on Cancel
cannot approve. Single in-flight execution; progress/status is native-owned.

Persist validated bounded intent before any dispatch. Storage failure stops pending
work; persist each outcome before dispatching another item. Cancellation stops
pending work only. Recheck cancellation and the native plan deadline after
filesystem revalidation and after Docker fresh checks, immediately before dispatch.
Eligible Docker plans must use the shared executor and persist their actual scoped
removal outcomes, including across restart. Outcomes: Removed, Skipped, Failed, Changed, Unknown. Once bytes
may have reached a tool or Trash may have run, never claim an error proves unchanged.
Retries require fresh planning/revalidation; never replay plans after restart.

Reuse private descriptor-relative atomic history (50 operations, 100 items each,
4 MiB maximum, mode 0600). Interrupted Pending becomes Unknown. Corruption stays
visible; clearing affects records only and is blocked during execution. Report
free-space observations separately, qualifying concurrent activity/APFS; selected
apparent sizes are not reclaimed bytes.

## Acceptance

Disposable fixtures/stubbed tools only. Mandatory actual platform Trash of eligible
npm/Cargo fixture, source/secret/outside sentinels intact, truthful recovery/history
across restart. Negative proof: forged/stale/expired plans, double submit, observed
target/ancestor/symlink change, excluded/out-of-scope references, overlap, Trash
failure/no fallback, partial/cancel, fresh retry, corrupt/interrupted/failed history.
Tool fixtures prove exact non-force scope, clean/locked/main/untracked/ignored guards,
local context/daemon/reference changes and partial/ambiguous errors. App tests do not
prove daemon atomicity. Native UI/IPC and frontend confirmation/outcome proof required;
CUA capture gaps remain explicit and do not waive automated native proof.

## Delivered native transport and proof

`cleanup_review({ selections })` resolves 1–100 exact native references, returns
per-item method/explanation and an optional native plan ID/expiry. One plan per
service is retained; another review invalidates the old plan. Mixed removal methods
require separate review/confirmation; unsupported selections never expand the plan.
`cleanup_execute({ planId, confirmed: true })` consumes that plan; `cleanup_status`
returns native progress; `cleanup_cancel` requests stopping pending work. Only the
local main window receives these scoped commands. There is no path/command endpoint.

Whole target subtrees are compared through the fingerprint above against the scan
measurement and exact plan metadata, plus ancestor identities and stamped recognizer
evidence (native identities, sizes, modification/ctime nanoseconds). Interior symlinks are observed without
following them and carried as links by Trash; target/ancestor symlinks cannot redirect
validation. Protected or incompletely measured descendants block cleanup. Exclusion
or registration changes cannot grant old-plan authority. Shared aliases remain
conservative inspection-only. Revalidation is bounded, and not an atomic platform
transaction. Git status/registry checks reuse the existing confined recognizer;
Git removal uses the fixed native executable, isolated configuration and no force.

Docker removal currently supports only Docker 28 with API 1.48 and classic `overlay2`
storage, exact lowercase sha256 IDs, empty tags/digests and zero observed image and
container references. The fixed local endpoint, daemon ID/version and observed socket
identity are rechecked before one DELETE with no retry, proxy, redirects or fallback.
Other versions/backends and referenced/tagged images remain inspection-only. Local
proxy/backend changes and reference arrivals between checks and dispatch remain
residual risks; the older daemon research is not claimed disproven. HTTP errors,
conflicts, truncated/malformed success or incomplete deletion evidence are Unknown,
never proof of unchanged state. Only a complete response naming the selected ID
Deleted supports Removed; layer records are not extra reclaimed images/bytes.

Operation intent is persisted before dispatch; every outcome is persisted before
another item starts. Write failure stops pending work and is visible in live results.
The old pending on-disk record becomes Unknown after restart. Live native progress
is distinguished from interrupted history. Free-volume observations are stored
separately from item measurements and are not attributed to cleanup. Moving into
Trash generally retains storage until Trash is emptied outside this app.

Current verification:

Shared-executor review fixes: the native Docker plan → execution → restarted-history
regression first reproduced Changed instead of Removed. Deterministic callbacks and
controlled clocks also reproduced dispatch after cancellation during filesystem
revalidation and DELETE after cancellation during Docker fresh checks. The executor
now dispatches eligible Docker plans through the checked removal path and rechecks
pending cancellation/expiry after revalidation; Docker repeats the guard immediately
before DELETE. Both cancellation and expiry cases pass without sleeps. Focused
`cargo test -p reclaim-desktop cleanup --locked` (16 tests) and
`cargo test -p reclaim-desktop docker::removal --locked` (4 tests) passed, along with
`cargo clippy -p reclaim-desktop --all-targets --locked -- -D warnings` and Rust
formatting. Docker proof uses only stub transport, not live destructive operations.

- `cargo test -p reclaim-desktop cleanup::execution --locked`: actual platform Trash
  of npm dependencies and Cargo output; intact source/secret/outside sentinels;
  plan forgery/expiry/confirmation/replay, observed substitutions, cancellation,
  partial errors/fresh retry, pre-dispatch and post-dispatch persistence failures.
- `cargo test -p reclaim-desktop linked_git_worktree --locked`: disposable Git
  registry fixture, main/locked/dirty/untracked/ignored rejection and actual clean
  supported non-force removal; committed main-repository source remains intact.
- `cargo test -p reclaim-desktop docker::removal --locked`: stub-only exact fixed
  DELETE scope; fresh context/daemon/tag/container rejection; partial/error response
  remains Unknown and never retries. No live Docker destructive verification.
- `cargo test -p reclaim-desktop native_ipc_plan_confirmation --locked`: mocked
  Tauri transport to native review/confirmation, actual platform Trash, replay
  rejection and restarted history. Only history-directory injection/fixture dispatch
  instrumentation differ from production; no frontend-authorized paths.
- `rtk vitest run src/cleanup/cleanup.test.tsx`: selection/eligibility, safe initial
  focus, Cancel without dispatch, exact-target confirmation, irreversible consent,
  duplicate suppression, cancellation and outcomes/persistence error presentation.
- `npm run build:web` followed by
  `cargo run -p reclaim-desktop --example cleanup_fixture --features tauri/custom-protocol --locked -- --smoke`:
  actual production React + native WebView IPC passed selection → eligible review →
  Cancel-focused confirmation (Cancel first proved no dispatch) → actual fixture
  Trash → truthful outcome/history after native service restart. Native assertions
  verified source/secret/outside sentinels intact. This leaves only disposable QA
  artifacts in platform Trash; it never enumerates or empties the user's Trash.

The fixture overrides history storage to a disposable directory for **all** history
and execution commands, so QA never writes production history. Its picker offers
only synthetic roots. Without `--smoke`, no scan/removal starts automatically.
Native CUA now verifies actual selection, Cancel focus, confirmation, Trash outcomes
and history in 1440×1000 dark and light captures. Final captures are attached to the
task (`reclaim-cleanup-review-final.png`, `reclaim-cleanup-confirm-final.png`,
`reclaim-cleanup-outcomes-final.png`, `reclaim-cleanup-outcomes-light.png`). The initial
assetless manual fixture was corrected by rebuilding with `tauri/custom-protocol`
after tests. A compact resize attempt did not change the 1440×1000 window; compact,
200% enlargement and VoiceOver speech are not claimed. Finish review found no
material visual defects; its stale PRODUCT.md status finding was corrected.

`rtk npm run check` passed formatting, lint, TypeScript, 49 frontend and 68 Rust tests.
After the final copy/header adjustment, the 5 cleanup interaction tests and web build
passed again. A subsequently reproduced pre-worker cancellation race was fixed by
serializing cancellation with plan claiming and invalidating unclaimed plans;
`cargo test -p reclaim-desktop cleanup::execution --locked` passed all 7 execution
tests, including the added regression, and final all-target Clippy passed. Earlier
passing proof for unchanged tests is retained (69 total Rust tests now exist).
`rtk npm run build` produced the local ad-hoc signed application after the fix;
notarization and hosted CI are manager-owned delivery steps, not local claims.

## Historical partial-delivery record

The record below describes `bcd60a1` and its former stricter policy. It is preserved
as historical evidence, not current eligibility or a blocking concurrency requirement.

### Historical implementation acceptance record (2026-10-27)

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

### Historical partial delivery and native contract

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

#### Historical missing mutation guarantees

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

#### Historical verification of partial delivery

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
