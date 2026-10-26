# Reclaim

Understand where disk space went before deciding what to remove. Reclaim aims to
help developers and people using coding tools connect storage consumption to
projects and tools, understand removal consequences, and make informed reclamation
decisions.

**Current status: early desktop scaffold only.** The app displays its name, version
and desktop connection state. It does not scan files, measure storage, recognize
artifacts or clean anything. Read-only exploration, artifact recognition and
limited cleanup are future work; deletion and recovery policies are not finalized.

Reclaim is macOS-first, built with Rust, Tauri 2, React, TypeScript and Vite.
Windows and Linux are future targets. There is no supported downloadable release.

## Run from source

Requirements:

- macOS 11 or newer; native verification so far covers Apple Silicon macOS 27.0.1,
  not older macOS versions or Intel.
- Xcode Command Line Tools and macOS SDK: `xcode-select --install`.
- Node.js 22 LTS, version 22.12 or newer within that major; `.nvmrc` pins 22.23.3.
- npm 10, the supported frontend package manager.
- Rust installed through rustup; `rust-toolchain.toml` selects Rust 1.90.0,
  rustfmt and Clippy.

Clone the repository, then run from its root:

```sh
git clone --branch dev https://github.com/sajjad-fatehi/reclaim.git
cd reclaim
npm ci
npm run dev
```

Keep `localhost:1420` free. Development opens a native Tauri window; Control-C
stops it. `npm run dev:web` starts only Vite; native metadata is unavailable in
a normal browser.

```sh
npm run check
npm run format
npm run build
open target/release/bundle/macos/Reclaim.app
```

`check` runs formatting checks, ESLint, strict TypeScript, frontend tests and Rust
Clippy. `format` writes formatting edits. `build` creates a locally ad-hoc-signed
macOS app, without notarization; it is not a public distribution artifact.
See [local development](docs/development/local-setup.md) for authoritative setup
details and [scaffold verification](docs/specs/desktop-scaffold.md) for evidence
and platform limits.

## Repository map

| Path | Ownership |
| --- | --- |
| `src/` | React presentation and native connection state |
| `src-tauri/` | Native lifecycle, configuration and permissions |
| `crates/reclaim-core/` | Tauri-independent Rust domain core, currently empty |
| `docs/` | Product direction, design proposals, specifications and development guidance |
| `Cargo.toml`, `Cargo.lock` | Rust workspace and locked dependency resolution |
| `package.json`, `package-lock.json` | Frontend scripts and locked dependency resolution |

## Participate

Start with [documentation](docs/index.md), [planned product behavior](docs/product.md)
and [contribution guidance](CONTRIBUTING.md). Small, focused contributions are
welcome. Feature branches and pull requests target `dev`.

Follow the [code of conduct](CODE_OF_CONDUCT.md). Omit sensitive file paths,
file contents and credentials from reports. Read [security reporting limitations](SECURITY.md)
before discussing vulnerabilities publicly.

## License

[MIT](LICENSE), copyright (c) 2026 Sajjad Fatehi.
