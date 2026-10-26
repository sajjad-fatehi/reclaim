# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

The interface uses web technology inside a Tauri desktop application. Initial delivery targets macOS; Windows and Linux are future targets, not supported releases.

## Stack

Rust core, Tauri 2 desktop adapter, React, TypeScript and Vite frontend. The user explicitly approved a WebView interface after discussing native alternatives.

## Users

Developers and people using coding tools who need to understand unexplained disk consumption, including people unfamiliar with generated development artifacts.

## Product Purpose

Help people find where disk space went, understand which items deserve inspection, and assess the consequences of removing them. Success means an understandable account of disk usage and informed reclamation decisions.

## Positioning

Connect filesystem usage to its owning development tool or project and explain removal consequences. Recognized development artifacts and general filesystem exploration serve the same investigation.

## Operating Context

Users investigate storage pressure and broad operating-system storage categories that do not identify actionable paths. Relevant examples include dependencies, builds, logs, package stores, runtimes, editor data, coding-agent worktrees and containers. These are candidate integrations, not implemented coverage.

## Capabilities and Constraints

- The shell exposes Overview, Explore, Development, Review & Clean and Settings. First-run scope controls, measured Overview and one shared scan session consume the native read-only producer. Appearance and scan settings share validated native local preferences. Explore, Development and cleanup views remain pending.
- Development has related project and tool views; shared usage must not be counted twice.
- On first use, show suggested scan locations for review and editing. Scanning starts only after the user explicitly starts it.
- Show scan coverage, inaccessible locations and unknown usage; never portray partial coverage as complete.
- Explain identity, ownership, size, removal consequences and evidence limits for individual findings.
- Age does not prove disuse or safe removal. Directory size does not necessarily equal recoverable physical space, including on APFS.
- Protect uncommitted worktree changes and persistent container data. Promise recovery only where supported.
- Initial sequence: read-only exploration, a small evidence-driven set of recognizers, then limited explainable cleanup.
- Duplicate detection, app uninstalling, RAM cleaning and automatic cleanup are deferred.
- Product copy and repository documentation are English only initially.
- Application implementation is authorized. The shell, native read-only scanner, first-run scanning, shared session, native preferences and measured Overview are implemented. Explore, Development and cleanup remain separate feature tasks.

## Brand Commitments

Product name: Reclaim. Open-source project; preserve the existing MIT license and human copyright.

The user explicitly wants expressive icons, animations, transitions and motion to make the interface engaging. Treat these as part of the product's design quality, alongside readable data and accessible interaction. Detailed choreography remains a design proposal until implemented and reviewed.

The user's latest visual brief replaces the former Space Layers aesthetic: warm, modern, playful/fantastical and polished, with CleanMyMac as a visual-quality reference. Preserve Reclaim's own features, identity, truthful data and safety behavior. See [the verified reference packet](docs/design/warm-reference.md).

## Evidence on Hand

The repository began with its MIT license and now includes a Rust/Tauri/React scaffold and working desktop shell. Existing local reference script: `~/bin/reclaim`, a macOS development-artifact cleanup script outside this repository. Its commands are not an approved deletion policy. Product decisions are recorded in shared project notes. Shell acceptance and native evidence are recorded in [the shell specification](docs/specs/features/application-shell.md). The [scan foundation](docs/specs/features/scan-foundation.md) records implemented native scope grants, bounded read-only measurement, cancellation, transport and isolated fixture evidence; it now feeds the production Overview through the shared scan provider. The [Overview contract](docs/specs/features/overview-and-scope.md) records settings/session ownership and native synthetic-fixture evidence. No user-data scan results are claimed.

## Product Principles

- Explain consumption before suggesting action.
- Keep unknowns and evidence limits visible.
- Make ownership and removal consequences understandable.
- Preserve user data and trust through explicit, scoped actions.
- Prefer a small complete system over speculative flexibility.

## Open Decisions

The latest warm CleanMyMac-inspired direction supersedes the earlier Space Layers aesthetic. Design specifications live in [docs/design/index.md](docs/design/index.md). The built shell system is recorded in [DESIGN.md](DESIGN.md), with acceptance and explicit evidence limits in the shell specification. The [measured Overview and shared session](docs/specs/features/overview-and-scope.md) are implemented and documented; Explore, Development and cleanup remain planned. The native producer retains its own verified contract.
