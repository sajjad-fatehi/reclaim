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

`check` runs Prettier, rustfmt, ESLint, strict TypeScript, focused frontend tests and workspace Clippy. Individual commands are `npm run format:check`, `npm run lint`, `npm run typecheck`, `npm test` and `npm run check:rust`. Formatting targets application source/configuration and Rust files, not existing product/design documents, generated outputs or lockfiles. There is no core behavior requiring Rust unit tests yet.

## Local production build

```sh
npm run build
open target/release/bundle/macos/Reclaim.app
```

Build uses locked Rust dependencies, compiles local frontend assets and bundles a macOS `.app` with ad-hoc signing. No paid Apple identity, notarization or release credentials are required. This is a local development artifact, not a notarized downloadable release; Gatekeeper/distribution behavior is not promised. No DMG or updater is configured.

## Ownership and security

- `crates/reclaim-core`: dependency-free, Tauri-independent domain crate, intentionally empty until real domain behavior exists.
- `src-tauri`: native lifecycle/configuration; metadata uses built-in Tauri facilities, no custom command or artificial core interface.
- `src`: React presentation and native connection state. One stylesheet holds current theme roles.
- Root Cargo workspace and lockfile own Rust resolution; npm lockfile owns frontend resolution.

Only name/version permissions are granted to the local `main` window. Production CSP allows bundled scripts/styles/images and native IPC, with no remote content or external network access. Development CSP additionally allows Vite's local HMR transport and injected styles/preamble. No filesystem, shell, scanner, cleanup, helper or telemetry capability exists.

See [scaffold specification and verification](../specs/desktop-scaffold.md) for acceptance evidence and limits. Planned product/design documents remain future behavior, not scaffold capabilities.
