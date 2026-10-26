# Development artifacts, projects and worktrees

Follow [shared requirements](index.md). Consume the actual scanner identity/measurement contract and publish the actual typed finding/ownership/consequence/eligibility contract for Docker, Development views and cleanup consumers.

## Required coverage

Implement evidence-based read-only recognition for these initial families. A recognizer must have representative fixtures and explicit non-matches; a string equal to a folder name is insufficient ownership evidence.

| Family | Initial inventory and explanation |
| --- | --- |
| JavaScript projects | node_modules; Next/Turbo/Nuxt/SvelteKit outputs; project manifests and source relationship |
| Python projects | .venv/venv with environment metadata; project ownership; distinguish user source from generated environment |
| Build/log data | Rust target and verified framework outputs; project logs when evidence supports the classification, not every .log file |
| Package stores | npm, pnpm, Yarn, Bun, pip and uv stores/caches; distinguish shared stores from per-project dependencies |
| Runtimes/toolchains | nvm Node versions, Go module/build caches, Cargo registry/cache and Homebrew cache inventory; never call installed runtimes removable merely because old |
| Editors/coding tools | VS Code, Cursor, JetBrains and identifiable Claude/Codex worktrees/artifacts; separate cache/logs from workspace history, chats, credentials, settings and unsaved state |
| Git worktrees | Standard Git registry plus Claude/Codex worktree locations verified as actual worktrees; associated repository, branch, main/linked status and changes |
| Temporary data | User-selected temporary locations including the macOS /tmp alias; inventory and contextual explanation, no blanket cleanup or age-based safety claim |

## Behavior and safety

- Discover configured tool locations with supported bounded mechanisms when available; do not assume every machine uses default paths. Do not invoke install/update/prune/cleanup commands during detection or auto-start missing tools.
- Recognize project boundaries from verified metadata. Assign native identities and provenance/confidence, distinguish shared/project-owned storage, deduplicate nested/linked findings and avoid counting parent and recognized child again in totals.
- Determine worktree change status without mutating Git. Untracked/ignored/unknown state is not clean; missing tools, corrupt metadata, detached or locked worktrees remain inspectable with uncertainty. Do not expose file contents or execute repository hooks.
- Explain regeneration/download cost and why a finding deserves inspection. Age is a hint only. Recognition and cleanup support are separate fields; unsupported/uncertain cases remain inspection-only.
- Missing tools/paths and malformed editor/workspace/package metadata produce scoped unavailable/unknown results, not application failure or manufactured zero size. Do not scan outside explicitly authorized roots to find global caches.

## Acceptance and proof

- Every family above has fixture-backed positive/negative behavior and explicit unsupported/configuration limits documented in a coverage matrix; no family may be silently omitted.
- Shared stores and nested artifacts are counted once; false-positive user folders, project source, credentials and persistent editor history never receive a cleanup-safe label.
- Clean/dirty/untracked/ignored/locked/detached worktree fixtures and malformed/missing-tool cases pass focused tests. Native methods operate read-only and remain bounded/cancellable with the scan session.
- Publish concrete contracts and actionable consequence explanations; required checks pass. No cleanup methods are executed in this task.

## Implementation acceptance record

The producer extends the existing scan worker and `ScanData`; no independent scan,
frontend path authority, registry plugin system or cleanup operation is introduced.
Recognition reads only bounded, identity-checked, no-follow metadata already present
in the authorized scan. Cancellation and partial measurement remain explicit.
Findings reference scanner IDs; ownership and accounting are separate from cleanup
support. All findings require inspection; regenerability never authorizes deletion.

Acceptance proof will include table-driven positive/negative fixtures for every named
family, malformed/configured/out-of-scope metadata, shared/nested accounting, protected
editor/source/secrets, and disposable real Git worktrees covering every required state.
Required gate: `npm run check`. No UI changes, destructive actions or tool installation.

## Delivered native contract

Authoritative types: `reclaim_core::recognition::{Recognition, Artifact, Family,
Nature, Evidence, Confidence, CleanupSupport, Worktree, ChangeState, Diagnostic}`.
They serialize camelCase fields and snake_case enums. Byte totals are decimal
strings; no paths, metadata contents, tool stdout or credentials are returned.
`Artifact.findingId`, `projectId`, evidence IDs and worktree repository/metadata IDs
refer to the same generation's scanner records. Scanner identities, measurements,
issues and timestamps remain authoritative; an ID is not cleanup authority.

Recognition runs after metadata traversal in the **same background scan worker**,
before terminal status. Existing `scan_cancel` cooperatively cancels both phases.
No launch-time scan, separate session, executable endpoint, install/prune command
or destructive operation is added. `ScanData.recognition` is available through
`ScanService::with_terminal_scan` to native consumers. `recognize` accepts native
records only; consumers must not synthesize records from frontend claims.

The new local-main-window command is:

`recognition_page({ generation, offset, limit })` → `RecognitionPage { generation,
finished, cancelled, limited, offset, nextOffset, findings, diagnostics,
diagnosticsTruncated }`. It rejects running/stale sessions and invalid offsets;
limit is 1–200. It shares the scanner's 100 ms retrieval throttle and error codes.
Diagnostics (at most 256, with truncation flag) appear only on the first page.
`nextOffset: null` ends findings. There are at most as many findings as scan records.
A finished recognition can still contain scoped unknown/unavailable diagnostics;
`finished` means analysis reached its end, not complete filesystem coverage.

Each artifact carries family, nature, nearest verified `projectId` (null for shared
stores/unassigned data), confidence, bounded evidence references/methods,
actionable consequence, `regenerable`, `complete`, exclusive byte contributions,
optional project ownership totals and optional worktree state. `regenerable` means
a known generation/download relationship, **not** permission to delete or proof
that local edits/persistent data are absent. `cleanupSupport` is currently always
`inspection_only`; this producer implements no removal strategy. Source/project,
runtime, editor state, secrets, unknown worktrees and temporary data remain protected.

### Accounting and ownership

`exclusiveLogicalBytes` / `exclusiveAllocatedBytes` sum scanner regular-file
contributions attributed to the nearest recognized ancestor (or the file itself).
Separately recognized children are excluded from parent exclusive amounts. Scanner
hard-link aliases add no bytes. Shared stores have no project owner; cached package
manifests are not promoted to source projects. Unknown storage outside any finding
is unclassified, not manufactured zero usage.

Only verified project/worktree findings carry `ownedLogicalBytes` /
`ownedAllocatedBytes`. These sum nearest-project contributions, including that
project's recognized artifacts and excluding nested projects. These are alternate
ownership summaries, **not additive** with exclusive totals. Neither basis estimates
physical recovery. Partial scan/recognition flags qualify every total; cancellation
can leave incomplete attribution. Worktree Git metadata may belong to the main
repository even when its linked checkout is elsewhere.

### Scope, reads and resource limits

The measurement walker remains descriptor-relative and metadata-only. Its new
recognition phase selectively reads known manifests/configuration/cache markers
already retained in the scan: no arbitrary source, chat, secret or history-content
search. Each component is reopened no-follow and checked against scanned native
identity; metadata reads require a complete regular-file record, cap at 64 KiB and
recheck size/change timestamps. Content is never emitted. A replaced/unavailable
root or entry becomes a scoped diagnostic, never authority to follow another path.

Recognition retains a bounded path/child index over the scanner's at most 100,000
records (at most 16 MiB of indexed native path bytes), at most 4,096 cached metadata reads and approximately 8 MiB of metadata
(one final read may exceed the total budget by at most 64 KiB). Analysis has a
15-second cooperative budget, checked between records/reads; a running Git call
can add at most its two-second deadline plus OS scheduling/syscall time. No hard
latency guarantee covers blocked filesystem calls. Cancellation/limits retain
findings and mark analysis incomplete. No unbounded subprocess output is retained.

Git state uses only the installed Command Line Tools Git binary at
`/Library/Developer/CommandLineTools/usr/bin/git`; it never invokes `/usr/bin/git`
(the installer shim), `xcrun`, hooks, package tools or missing-tool installers.
macOS `sandbox-exec` confines file content reads to selected native roots and
system runtime directories; ancestor metadata is allowed only for path resolution.
Writes are denied except `/dev/null`; network and further executable launches are
denied. Git receives a cleared environment, disabled optional locks/fsmonitor/hooks,
no external attributes file, no renames and no submodule recursion. Global/system
Git config is disabled. Unsupported repository config (includes, filters, worktree
redirects, extensions, etc.) returns unknown before launch. Captured status output
is limited to 64 KiB, is never exposed, and a timed-out/cancelled child is killed
and reaped. Missing CLT Git/confinement, unsupported platform, sandbox denial,
output limits or Git failures yield scoped unknown status, not clean. This depends
on macOS's existing Seatbelt launcher; no unsafe Rust, helper install or new shell
capability is introduced.

HEAD and registry backlinks must resolve to scanned native records. Linked
worktrees require `.git` → registry entry, `commondir`, and reciprocal `gitdir`;
main repositories require their own metadata. Branch, detached/main/locked state
remain explicit. Status precedence is dirty, untracked, ignored, clean; unknown
covers incomplete/locked/unavailable states. Ignored or untracked data is never
clean. Status is an observation, not an atomic snapshot or deletion eligibility.

## Coverage and supported configuration matrix

Every row has positive and name-only/invalid negative fixtures in
`recognition/tests.rs`; Git additionally uses disposable real repositories/worktrees.
No tool is required for filesystem-family inventory.

| Family | Delivered evidence | Supported custom locations and limits |
| --- | --- | --- |
| JavaScript projects/dependencies | Parsed `package.json`; npm `.package-lock.json`, pnpm `.modules.yaml`, Yarn integrity, or declared-and-installed package manifest | Nearest manifest boundary; nested source projects preserved; generated/vendor descendants not promoted to projects. Lock interpretation is recognition only, not dependency resolution. |
| Python | Parsed `pyproject.toml` project/tool table plus immediate `.venv`/`venv` `pyvenv.cfg` with interpreter home | Config-only/legacy `setup.py` projects and arbitrary venv names remain inspection-only/unclassified; no interpreter execution. |
| Framework/Rust outputs | Declared Next/Turbo/Nuxt/SvelteKit dependency plus output marker; Cargo manifest and `target/.rustc_info.json` | Conventional project-local outputs only; custom build output paths and arbitrary `dist` directories remain unclassified. |
| Project logs | Parsed npm script explicitly redirects into project `logs`/`.logs` containing `.log` files | Arbitrary `.log` files, Python/Rust logging config and runtime command interpretation are unsupported; never assume every log is disposable. |
| npm | `.npm/_cacache/{index-v5,content-v2}` or valid cache signature | Scoped `.npmrc` `cache`, `npm_config_cache`; configured directory inventory is contextual, not proven regenerable. |
| pnpm | `pnpm/store` or `.pnpm-store` with `v3/files`, or cache signature | `PNPM_STORE_DIR`; other store versions require cache signature/configured inventory. No pnpm command/config execution. |
| Yarn | Conventional Yarn/v6 or `.yarn/cache` plus package archive/cache entry or signature | Scoped simple `.yarnrc.yml` `cacheFolder`, `YARN_CACHE_FOLDER`; no general YAML evaluation or v1 rc command interpretation. |
| Bun | `.bun/install/cache` with versioned package manifest or signature | Scoped `bunfig.toml` `[install.cache] dir`, `BUN_INSTALL_CACHE_DIR`; no Bun execution. |
| pip | `.cache/pip` or macOS `Caches/pip` with `http-v2`, or signature | Scoped `pip.conf` `cache-dir`, `PIP_CACHE_DIR`; no Python imports or pip execution. |
| uv | `.cache/uv` or macOS `Caches/uv` with `wheels-v5`, or signature | Scoped `uv.toml` `cache-dir`, `UV_CACHE_DIR`; other schemas remain contextual configured inventory. |
| nvm | `versions/node/<version>` with `bin/node` and Node version header | Custom nvm root retains this layout. Runtime inventory only; no old-version removal inference. |
| Go | `go/pkg/mod/cache/download`; build-cache README signature | `GOMODCACHE`, `GOCACHE`; `go env` is not executed. Configured location inventory remains contextual. |
| Cargo | `.cargo/registry/{cache,src,index}` | `CARGO_HOME` inventory is contextual and protects credentials/toolchains; no Cargo command. |
| Homebrew | `Caches/Homebrew/downloads` or cache signature | `HOMEBREW_CACHE`; inventory only, no brew/prune command. |
| VS Code/Cursor | Conventional macOS profile, parsed `User/settings.json`, runtime cache/log layout | JSON settings only; JSONC, portable/custom profiles require future explicit support. Cache/log children remain inspection-only; User/history/backups/workspace state protected. |
| JetBrains | JetBrains product profile plus `options/ide.general.xml` or `.home` | IntelliJIdea/PyCharm/WebStorm/RustRover profile prefixes; custom properties/other products not evaluated. Profile/cache/log inventory only. |
| Claude/Codex | Scoped `.claude/settings.json` or `.codex`/`.Codex/config.toml`; real worktrees verified separately | Chats, sessions, auth, settings, workspace history never read for classification. Folder name alone does not verify a worktree. |
| Git worktrees | HEAD + repository metadata; linked registry/common/backlink validation; confined porcelain status | Main/linked, Claude and Codex layouts; clean/dirty/untracked/ignored/locked/detached fixtures. Out-of-scope registry, malformed metadata, missing tools/config extensions are inspectable unknown. |
| Temporary locations | Explicitly scanned root equal to canonical `/private/tmp` (`/tmp` picker alias) or process `TMPDIR` | Custom `TMPDIR` must itself be a selected scan root. A child merely named `tmp` is not recognized; no blanket temp cleanup. |

Configuration files are parsed only if scanned; referenced directories must also
be retained scanner directories. Relative paths resolve against their config file;
`..` is normalized without escaping consent. No shell expansion, `~`, includes,
command interpolation or searches outside scope. Environment locations are captured
from the native process and matched only to already scanned directories. Absent paths
produce no size claim; out-of-scope/malformed metadata produces bounded diagnostics.
Unrecognized names/content remain normal scanner findings, not cleanup-safe artifacts.

## Verification evidence

Local Apple Silicon macOS proof, using disposable/synthetic fixtures only:

- `npm ci`: installed locked repository dependencies; audit reported zero vulnerabilities.
- `npm run check`: formatting, ESLint, strict TypeScript and two frontend tests passed;
  initial Rust Clippy stopped on a cloned test slice. Fixed by borrowing the slice.
- `npm run check:rust && npm run test:rust`: workspace/all-target Clippy passed;
  21 core and seven native tests passed, including real registry/backlink states,
  guarded transport, serialization, paging, stale generations and window denial.
- After the final metadata-budget regression edit,
  `cargo clippy -p reclaim-core --all-targets --locked -- -D warnings` passed and
  `cargo test -p reclaim-core --locked` passed **22** core tests. Malformed UTF-8
  metadata consumes the same read budget as valid metadata; a synthetic long-path
  index stops at its memory budget. Unchanged native/frontend evidence is reused.
- `npm run build`: production web build, locked release Rust build and ad-hoc signed
  `Reclaim.app` bundle passed. No notarization/public distribution is claimed.
- Local link validation passed eight relative links in changed authoritative docs;
  `rtk git diff --check` passed.

The Git fixtures cover main/linked and actual Claude/Codex worktree locations,
clean/dirty/untracked/ignored/locked/detached state, unsupported config, outside-scope
registry and unchanged index bytes. Additional launcher fixtures verify quoted
scope paths, denied unselected roots, missing executable/confinement and cancellation.
Family tests cover configured stores, malformed manifests/editor metadata,
symlink/identity changes, nested projects, shared hard links, protected state and
explicit temporary-root context. No family requires a real package cache or editor
installation. No user data, Docker, deletion, prune, hooks or installer launch is
used by discovery verification. No UI changes or additional WebView smoke run;
native command behavior is verified with the existing Tauri dispatcher test adapter.
