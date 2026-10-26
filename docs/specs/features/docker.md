# Optional local Docker inventory

Follow [shared requirements](index.md) and the delivered recognition contract. Docker is an optional local tool, not a prerequisite to launch Reclaim.

## Required behavior

- Detect a supported installed Docker CLI and an available local daemon without starting Docker, pulling images or changing context. Remote contexts/endpoints are inspection-unavailable for this local-disk product; do not silently contact remote hosts.
- Use bounded typed commands with arguments, timeouts, cancellation and validated structured output; never interpolate a shell command from user input. Do not collect container secrets, environment variables or file contents.
- Report available local image/container/build-cache/volume inventory and ownership/reference state, qualifying shared layers, active use and measurement uncertainty. Report unavailable per-item metrics instead of allocating an aggregate arbitrarily.
- Distinguish persistent volumes and container writable layers from rebuildable images/caches. Mark persistent, referenced, running, ambiguous and unsupported resources as inspection-only.
- Attach validated identities and evidence to the common artifact model. Do not add Docker's virtual-disk apparent size to its internal layer totals or to counted filesystem parents; show the relationship and keep overlap explicit.
- Unsupported versions/alternative engines, malformed output, command timeout, absent CLI, stopped daemon and remote context each produce an honest state with remediation. Alternative engines may be represented as unsupported; no unverified compatibility claim.

## Acceptance and proof

- Stubbed process/transport fixtures cover success, shared layers, active/persistent resources, malformed/partial output, unavailable tool, stopped daemon, remote context and timeout/cancel.
- No engine startup, network fetch, container mutation or privileged generic shell endpoint is introduced. Persisted context is not changed.
- Document which metrics are exact, approximate, shared or unavailable and which supported Docker versions/formats were verified. Deliver real read-only inventory, not a static integration tile.
- Relevant checks pass and Development consumers receive the actual contract. Real Docker deletion is never used as verification.

## Implementation contract and acceptance record

The optional inventory is explicitly started through `docker_inventory`; cancellation
uses `docker_cancel`. No launch-time discovery occurs. The native adapter accepts no
path, endpoint or command arguments. Fixed Docker CLI locations and metadata-only
CLI calls identify the current local endpoint; only default/Desktop contexts and
standard Unix sockets are supported. Alternative endpoints remain unsupported.
Docker Engine 24–29 structured Engine API responses are supported; fixtures verify
formats, not live installations. Native GET-only transport reads `/version` and the
negotiated version's `/system/df`, with time, output and resource-count bounds.

Core owns parsing and shared artifact consequences. Docker items embed `Artifact`
with Docker identity/evidence, no project attribution and inspection-only cleanup.
Filesystem-exclusive byte contributions remain zero; separate optional decimal
metrics describe engine-reported logical/shared/writable sizes, never physical
reclaimability. Persistent volumes and writable layers are not regenerable. Image
layers and shared build caches are non-additive. Virtual disk overlap is explicit;
consumers must never add engine metrics to filesystem or disk totals.

Acceptance fixtures must cover four resource kinds, references/running/persistence,
shared sizes, null metrics, invalid IDs/negative metrics, malformed/partial replies,
version rejection, endpoint rejection, unavailable/stopped daemon, bounded output,
timeout and cancellation. Only fixed discovery calls and GET requests are allowed.

### Consumer API

Both commands require the local `main` window and `docker-read-only` capability.
`docker_inventory` takes no arguments, runs off the UI thread, and returns
`{ measuredAtMs, result: { Ok: { items, accounting } } }` or
`{ measuredAtMs, result: { Err: failure } }`. Concurrent requests reject `busy`.
`docker_cancel` takes no arguments and sets cancellation for the running request.
Snapshots are observations, not atomic daemon snapshots or cleanup tokens; callers
own stale presentation and must request a fresh inventory after external changes.
No discovery runs automatically and Docker failure cannot prevent filesystem scans.

Each item contains `artifact`, `kind` (`image`, `container`, `build_cache`, `volume`),
`identity`, nullable `references`, nullable `active`, `persistent`, nullable `shared`,
nullable decimal `logicalBytes`/`sharedBytes`, and `metricBasis`. Docker finding IDs
use a `docker:` namespace and are not scanner IDs/path grants. `projectId` is null:
labels, Compose files, environment and mounts are not collected for speculative
ownership. Container writable-layer bytes exclude image size; persistent volumes
can use external drivers, so presence does not prove local backing storage.
Numeric engine observations are exact reported integer byte values, but their
physical allocation/reclaimability is unknown. Null or Engine `-1` means unknown,
never zero. Shared image bytes and shared build records cannot be summed.

Native discovery checks `/usr/local/bin/docker`, `/opt/homebrew/bin/docker`, then
Docker.app's bundled CLI. Commands are `--version`, `context show`, and
`context inspect <default|desktop-linux> --format '{{json .Endpoints.docker.Host}}'`.
Environment is cleared except HOME, fixed PATH and LANG. `DOCKER_HOST` and custom
`DOCKER_CONFIG` are unsupported, rather than silently ignored. The current named
context must be `default` or `desktop-linux`; only `unix:///var/run/docker.sock` or
`unix://$HOME/.docker/run/docker.sock` passes. Docker Engine/Desktop platform names,
Engine/CLI major versions 24–29 and API versions 1.43–1.52 are accepted. Alternative
engines and nonstandard/rootless/remote sockets are explicitly unsupported.

GETs use macOS `/usr/bin/curl --disable` with a pinned Unix socket, no proxy,
redirects, config files, retry or network transport. There is no general curl/shell
endpoint. Discovery plus inventory has a 20-second total deadline; each GET has
2-second connection and 5-second request limits. Cancellation polling is 20 ms;
children are killed and reaped on deadline/cancel. Output is capped at 8 MiB per
command, inventory at 4096 resources. All four list keys must exist (null means
empty); invalid/missing identities, invalid states, duplicate IDs, unsupported
numeric types and truncated JSON fail closed without exporting partial items.
Unknown metrics remain null. HTTP/CLI stderr, raw JSON, labels and secrets are not
returned or logged. No filesystem path is attributed to engine bytes.

### Failure and remediation

| Failure | Consumer remediation |
| --- | --- |
| `unavailable` | Install supported Docker CLI or restore local tool access; filesystem scanning remains available. |
| `unsupported_context` | Choose a supported local Docker context outside Reclaim; remote/alternative endpoints are not inspected. |
| `unsupported_version` | Use supported Docker Engine/CLI/API versions. |
| `unsupported_engine` | Alternative engines are not verified; use supported Docker Engine/Desktop. |
| `daemon_unavailable` | Start/check Docker outside Reclaim, then retry; Reclaim never starts it. |
| `malformed` | Retry; if persistent, engine output is unsupported/incomplete. |
| `limited` | Inventory exceeded bounded output/resource limits; reduce inventory externally. |
| `timeout` | Check daemon responsiveness, then retry. |
| `cancelled` | Inventory cancelled; restart explicitly. |
| `busy` | Wait for existing inventory or cancel it. |
| `access_denied` | Only authorized local main window can request inventory. |

## Verification evidence

Synthetic/disposable fixtures on local macOS; no live Docker or user data used:

- `rtk npm ci`: locked dependencies installed; audit found zero vulnerabilities.
- `rtk npm run check`: formatting, ESLint, TypeScript, nine frontend tests and
  workspace/all-target Clippy passed. Rust tests initially exposed duplicate macOS
  plist embedding from two test contexts. Both IPC suites now reuse one generated
  test context; no runtime scanner behavior changed.
- `cargo test --workspace --locked`: 26 core and 14 native tests passed after that
  repair, including all seven Docker native tests, Unix-socket HTTP fixture,
  process kill/reap, bounded output, endpoint rejection and IPC guards.
- After adding one final stopped/unused/duplicate/count-limit core fixture,
  `cargo clippy --workspace --all-targets --locked -- -D warnings` passed;
  `cargo test -p reclaim-core --locked docker` passed all four Docker core tests.
  Unchanged 23 core and 14 native test evidence is reused.
- `cargo fmt --all -- --check`, `rtk git diff --check`, and relative specification
  link validation passed.

Engine/CLI versions and response formats above are fixture-supported, not a claim
that six real engine installations were exercised. No live daemon, app bundle,
WebView or Development UI proof was run for this backend-only delivery. Development
owns presentation of this consumer contract. No startup, pull, context mutation,
cleanup, deletion, secrets inspection or filesystem attribution was performed.

### Review repair acceptance: inherited stdout

A CLI wrapper can exit while a descendant retains stdout. Command completion must
require both stdout EOF and direct-child exit, while continuing deadline/cancel
checks. Pipe reads must be nonblocking; no reader thread or unconditional pipe
join may outlive the command budget. Preserve the output cap and direct-child
kill/reap on failure. Disposable wrappers must reproduce both deadline and
cancellation after parent exit; fixture descendants must be explicitly cleaned up.
No live Docker, additional shell IPC, parser or frontend changes are involved.

The repair reuses the existing locked `rustix` dependency directly in the native
adapter for safe nonblocking `fcntl`, matching recognition's Git pipe pattern.
No output-reader thread is created. The same loop drains at most 8 MiB plus one
limit-detection byte, observes direct-child exit and continues checking cancellation
and the total deadline until EOF. Both success prerequisites are required; an
exited wrapper with an open descendant pipe cannot report premature success.
Failures close the read pipe, kill the direct child if still alive, and reap it.
Only the directly spawned process is owned/reaped; arbitrary descendants are not
terminated. Fixtures explicitly kill their recorded isolated stdout-holder PID,
verify its disappearance, and retain a five-second natural-exit fallback. No
process-group signaling or broader process-management capability is introduced.

Polling remains 20 ms while idle and runs between bounded reads under continuous
output. These are cooperative checks, not hard real-time guarantees: process spawn,
OS scheduling, kill and reap still depend on the OS. A descendant cannot stall a
blocking pipe read or thread join. Regression bounds allow scheduling tolerance:
one-second remaining deadline returns within two seconds; cancellation after
verified parent exit returns within 250 ms. No live Docker is exercised.

Focused repair evidence (synthetic wrappers only):

- Before the runtime edit,
  `cargo test -p reclaim-desktop --locked inherited_stdout_after_parent_exit -- --nocapture`
  failed both regressions: commands returned `Ok("synthetic")` after the inherited
  pipe stayed open for five seconds, ignoring deadline and cancellation (5.02 s
  combined test duration). Each wrapper's direct-child exit was verified.
- After the edit, the same focused regressions passed (1.02 s combined duration).
- Final `cargo test -p reclaim-desktop --locked docker::tests` passed all nine native
  Docker tests (1.04 s), including output caps, direct-child kill/reap, inherited
  stdout deadline/cancellation, isolated holder cleanup, Unix GETs and IPC guards.
- `cargo clippy -p reclaim-desktop --all-targets --locked -- -D warnings`,
  `cargo fmt --all -- --check`, and `rtk git diff --check` passed.

Unchanged parser, scanner and frontend evidence above is reused. No live daemon,
production build, full frontend gate or additional UI verification was run.
