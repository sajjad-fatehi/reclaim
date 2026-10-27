# Local development

## Requirements

- macOS 11 or newer. Initial delivery and native verification target Apple Silicon macOS; other platforms are not delivery targets yet.
- Xcode Command Line Tools (`xcode-select --install`) with the macOS SDK.
- Node.js 22 LTS, at least 22.12; `.nvmrc` pins the verified version. npm 10 is the only supported frontend package manager.
- Rust through rustup. `rust-toolchain.toml` selects Rust 1.90.0 with rustfmt and Clippy automatically.

## Install and run

From repository root:

```sh
npm ci
npm run dev
```

Tauri starts Vite on `localhost:1420` and launches the native window. Keep that port free; the server fails rather than choosing another port. Stop development with Control-C. The window reads app name/version through Rust-backed Tauri IPC. `npm run dev:web` runs only the frontend; its native connection message will report unavailable when opened in a normal browser.

## Checks and formatting

```sh
npm run check
npm run format
```

`check` runs Prettier, rustfmt, ESLint, strict TypeScript, focused frontend/Rust tests and workspace Clippy. Individual commands are `npm run format:check`, `npm run lint`, `npm run typecheck`, `npm test`, `npm run test:rust` and `npm run check:rust`. Formatting targets application source/configuration and Rust files, not existing product/design documents, generated outputs or lockfiles. Run `cargo test --workspace --locked` for scanner, authority and IPC fixture tests.

## Local production build

```sh
npm run build
open target/release/bundle/macos/Reclaim.app
```

Build uses locked Rust dependencies, compiles local frontend assets and bundles a macOS `.app` with ad-hoc signing. No paid Apple identity, notarization or release credentials are required. This is a local development artifact, not a notarized downloadable release; Gatekeeper/distribution behavior is not promised. No DMG or updater is configured.

## Ownership and security

- `crates/reclaim-core`: Tauri-independent metadata traversal, identity, accounting, coverage and scoped development-recognition rules.
- `src-tauri`: native lifecycle/configuration, folder selection, process-local scope authority, bounded scanner/Docker IPC, cleanup eligibility and single-use plans, privileged removal dispatch and private operation history. App metadata still uses built-in Tauri facilities.
- `src`: React presentation and native connection state. One stylesheet holds current theme roles.
- Root Cargo workspace and lockfile own Rust resolution; npm lockfile owns frontend resolution.

Name/version and the narrow `scan-read-only`, `docker-read-only` and `cleanup-review` permissions are granted only to the local `main` window. The cleanup ACL exposes only `cleanup_review`, `cleanup_execute`, `cleanup_status`, `cleanup_cancel`, `cleanup_history` and `cleanup_clear_history`. Native code resolves opaque finding IDs into bounded, expiring, single-use plans; execution requires explicit confirmation and immediate scope/identity/safety revalidation. Supported removal uses macOS Trash or narrowly scoped non-force Git/Docker methods; no permanent-delete fallback exists. See [cleanup contract](../specs/features/review-and-cleanup.md) for eligibility, recovery and residual concurrent-change limits. Production CSP allows bundled scripts/styles/images and native IPC, with no remote content or external network access. Development CSP additionally allows Vite's local HMR transport and injected styles/preamble. No generic filesystem or shell permission, privileged helper or telemetry capability exists. Cleanup authority is limited to the native commands above; frontend paths, commands and sizes cannot authorize removal. The native dialog plugin is used from Rust only; its frontend dialog/filesystem permissions are not granted. No scan starts at launch. The same worker runs bounded read-only development recognition after measurement; `recognition_page` exposes terminal findings under the same permission. See [recognition contract](../specs/features/development-recognition.md) for known metadata reads and optional confined CLT Git status. See [scanner contract](../specs/features/scan-foundation.md) for scope grants, resource limits and coverage semantics.

See [scaffold specification and verification](../specs/desktop-scaffold.md) for acceptance evidence and limits. Unimplemented product/design capabilities remain planned; the scanner contract above describes the delivered native producer.

## Disposable native scanner smoke

```sh
npm run build:web
cargo run -p reclaim-desktop --example scan_smoke --features tauri/custom-protocol --locked
```

This example creates and removes its own synthetic fixtures, drives real macOS
WebView IPC through the production handlers and ACL, then exits with success or
failure. It never discovers or scans user locations. Set
`RECLAIM_SMOKE_KEEP_OPEN=1` to retain its labelled result window for a screenshot;
close the window afterward to release the disposable fixture. This is a development
example, not an application mode or production endpoint. Native folder selection
still requires a user gesture in a consuming UI; cancellation returns `null`.

## Disposable native cleanup fixture

From repository root, build frontend assets once, then launch the interactive fixture:

```sh
npm run build:web
cargo run -p reclaim-desktop --example cleanup_fixture --features tauri/custom-protocol --locked
```

The window is labelled **SYNTHETIC disposable cleanup fixture**. Its picker offers
only disposable roots; preferences and history use a temporary directory instead
of production storage. Nothing scans or removes automatically in interactive mode.
Use only these fixture roots:

1. Choose **Start scan**, wait for completion, then open **Explore**.
2. Search for `node_modules`, inspect it and choose **Add to review**.
3. Open **Review & Clean**, choose **Check eligibility**, then **Review confirmation**.
   Check the exact synthetic target, Trash consequences and initial focus on **Cancel**.
4. Choose **Cancel** to verify dismissal without execution. Reopen confirmation and
   choose **Confirm selected removal** to move the disposable directory to real
   macOS Trash. Inspect **Cleanup outcomes** and **Operation history**.

For the verified automated production-React/native-WebView IPC smoke flow, reuse
those frontend assets:

```sh
cargo run -p reclaim-desktop --example cleanup_fixture --features tauri/custom-protocol --locked -- --smoke
```

Smoke mode performs that selection/confirmation/Trash flow automatically, verifies
Cancel did not dispatch, reloads history through a fresh native service and checks
source, credential and outside-scope sentinels remain intact. It prints
`SYNTHETIC production React + native IPC + actual macOS Trash + restarted history PASS; source/secret/outside sentinels intact`
and exits with status 0 on success, or status 1 on smoke failure.

Close the interactive window when finished; temporary roots/configuration/history
are discarded on exit. Removed fixture items remain in macOS Trash; recover through
Trash / Put Back where available. Moving to Trash does not itself free their storage.
The fixture never enumerates or empties Trash. Do not use private files or live Docker
deletion for verification; Docker mutation tests use stub transport. Detailed proof
and remaining limits live in the [cleanup specification](../specs/features/review-and-cleanup.md).
