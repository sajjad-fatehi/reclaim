# Desktop scaffold

## Purpose and observable behavior

Establish a runnable macOS-first Reclaim application where none exists. One English window identifies this as an early development scaffold. It reads application name and version through Tauri's built-in metadata API; loading and failure states remain understandable. No storage measurements or cleanup controls appear.

## Scope and ownership

- Root Cargo workspace: `crates/reclaim-core` reserves Tauri-independent domain ownership without invented domain types or behavior. `src-tauri` owns native lifecycle, configuration and permissions.
- `src` owns React presentation; Vite builds local assets. No domain rules in the frontend.
- npm is the sole frontend package manager; commit `package-lock.json` and workspace `Cargo.lock`.
- Pin Rust 1.90.0; require Node 22.12+ within Node 22 LTS and npm 10. Record exact resolved versions and local commands in development guidance.
- Plain system-font layout uses existing Space Layers colors, follows system appearance and supports text enlargement. No new design system or animation dependency.
- Exclude scanner, cleanup, database, telemetry, localization, plugins, privileged helpers and release infrastructure. Preserve MIT LICENSE and existing design/product authority.

## Acceptance criteria

- `npm ci` installs reproducibly; documented `npm run dev` launches a native macOS window.
- `npm run build` produces a local unsigned macOS application without paid signing credentials.
- Repository scripts check Rust formatting/Clippy, frontend formatting/ESLint and strict TypeScript. Relevant tests cover metadata success and rejection; no tests for static scaffold text or empty core.
- Native metadata success displays actual app name/version; rejection displays an accessible useful message without leaking error details. Unmounting prevents stale state updates.
- Semantic headings, status announcements, readable light/dark contrast and usable small-window/200% text layout. No fake navigation or unsupported actions.
- Root ignore rules cover target, dependencies, dist, generated Tauri schemas, logs, macOS files, local environment secrets and worker worktrees. Source, assets, both lockfiles, shared configuration and safe `.env.example` paths remain eligible for tracking.

## Safety constraints

Only main local window receives `core:app:allow-name` and `core:app:allow-version`. No filesystem, shell, remote-content or custom native commands. Production CSP restricts resources to bundled assets and Tauri IPC; no remote connections, inline scripts, object embedding or frames. Development alone permits local Vite HMR. No private data is read or logged.

## Unresolved decisions

None blocking. Initial macOS deployment target is 11.0 (Tauri baseline); current host provides native proof, not proof on every supported macOS version. Local unsigned application bundle only; distribution/signing remains out of scope.

## Verification evidence

Verified on Apple Silicon macOS 27.0.1, Command Line Tools SDK 27.0, Node 22.23.3, npm 10.9.8 and Rust 1.90.0.

| Acceptance | Command or observation | Outcome |
| --- | --- | --- |
| Reproducible installation | `npm ci` | Passed; 239 packages installed, audit reported zero vulnerabilities. No peer-resolution bypass flags. |
| Application checks | `npm run check` | Passed: Prettier, rustfmt, ESLint, strict TypeScript, two metadata success/rejection tests, workspace Clippy with warnings denied. |
| Production build | `npm run build` | Passed; `target/release/bundle/macos/Reclaim.app`, 8.29 MiB, ad-hoc signature. Frontend JS 69.96 kB gzip; no external fonts or network resources. |
| Signature | `codesign --verify --deep --strict target/release/bundle/macos/Reclaim.app` | Passed. `codesign -dv` confirmed `Signature=adhoc`, no team identity. Notarization intentionally absent. |
| Native production runtime | Opened exact built `.app` through native UI tool | Rendered dark-mode scaffold; native accessibility tree exposes headings/body and actual `Reclaim 0.1.0 · Desktop connected`. Screenshot attached to task, result `94d09bfd-9a94-441d-b867-75d3a608a6be`. |
| Development command | `npm run dev` | Vite ready on localhost:1420; Cargo compiled and ran `target/debug/reclaim-desktop`. Stopped with Control-C. Native UI tool cannot select this unbundled binary, so direct visual evidence is from production bundle. |
| Ignore coverage | Python assertion loop calling `git check-ignore --no-index -q <path>` | Passed all 23 representative paths below. No secret files read. |
| Documentation/whitespace | Relative-link assertion for scaffold docs; `git diff --check` | Passed. MIT LICENSE unchanged. |

Ignored examples: `target/release/reclaim-desktop`, `node_modules/react/index.js`, `dist/index.html`, `src-tauri/gen/schemas/desktop-schema.json`, `debug.log`, `.DS_Store`, `src/._App.tsx`, `.env`, `.env.local`, and a child file inside each of `.Codex/worktrees/`, `.codex/worktrees/`, `.claude/worktrees/`.

Trackable examples: `Cargo.lock`, `package-lock.json`, `src/App.tsx`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/main.json`, `crates/reclaim-core/src/lib.rs`, `.env.example`, `.env.development.example`, `rust-toolchain.toml`, `.prettierrc.json`, `src/assets/icon.svg`. Example environment/asset paths need not exist to verify ignore behavior.

Resolved setup failures: Tauri requires `icons/icon.png` even when the bundle icon list is empty, so a plain teal placeholder icon is included. Tauri Cargo arguments require the `-- --locked` separator. Vitest 4 optional browser peer metadata caused npm's `Cannot read properties of null (reading 'edgesOut')`; compatible stable Vitest 5.0.5 installs cleanly without overrides or extra browser dependencies.

Limits: no hosted CI, macOS 11 hardware, Intel, Windows/Linux, VoiceOver session, 200% native text enlargement or native light-mode render verified. System light/dark styling uses established CSS media queries, semantic text and no motion. No supported distribution/signing release is claimed. Core intentionally has no domain API or artificial test.
