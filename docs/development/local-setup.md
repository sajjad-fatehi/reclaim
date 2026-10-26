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

- `crates/reclaim-core`: Tauri-independent metadata traversal, identity, accounting and coverage rules.
- `src-tauri`: native lifecycle/configuration, folder selection, process-local scope authority and bounded scanner IPC. App metadata still uses built-in Tauri facilities.
- `src`: React presentation and native connection state. One stylesheet holds current theme roles.
- Root Cargo workspace and lockfile own Rust resolution; npm lockfile owns frontend resolution.

Name/version and the narrow `scan-read-only` permission are granted only to the local `main` window. Production CSP allows bundled scripts/styles/images and native IPC, with no remote content or external network access. Development CSP additionally allows Vite's local HMR transport and injected styles/preamble. No generic filesystem, shell, cleanup, privileged helper or telemetry capability exists. The native dialog plugin is used from Rust only; its frontend dialog/filesystem permissions are not granted. No scan starts at launch. See [scanner contract](../specs/features/scan-foundation.md) for scope grants, resource limits and coverage semantics.

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
