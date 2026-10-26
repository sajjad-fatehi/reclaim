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

- Planned sections: Overview, Explore, Development, and Review & Clean. Minimal settings cover scan locations, exclusions and permissions.
- Development has related project and tool views; shared usage must not be counted twice.
- On first use, show suggested scan locations for review and editing. Scanning starts only after the user explicitly starts it.
- Show scan coverage, inaccessible locations and unknown usage; never portray partial coverage as complete.
- Explain identity, ownership, size, removal consequences and evidence limits for individual findings.
- Age does not prove disuse or safe removal. Directory size does not necessarily equal recoverable physical space, including on APFS.
- Protect uncommitted worktree changes and persistent container data. Promise recovery only where supported.
- Initial sequence: read-only exploration, a small evidence-driven set of recognizers, then limited explainable cleanup.
- Duplicate detection, app uninstalling, RAM cleaning and automatic cleanup are deferred.
- Product copy and repository documentation are English only initially.
- Current design request is documentation only: no prototype, scanner or cleanup implementation.

## Brand Commitments

Product name: Reclaim. Open-source project; preserve the existing MIT license and human copyright.

The user explicitly wants expressive icons, animations, transitions and motion to make the interface engaging. Treat these as part of the product's design quality, alongside readable data and accessible interaction. Detailed choreography remains a design proposal until implemented and reviewed.

The user's latest visual brief replaces the former Space Layers aesthetic: warm, modern, playful/fantastical and polished, with CleanMyMac as a visual-quality reference. Preserve Reclaim's own features, identity, truthful data and safety behavior. See [the verified reference packet](docs/design/warm-reference.md).

## Evidence on Hand

The repository initially contains only its MIT license. Existing local reference script: `~/bin/reclaim`, a macOS development-artifact cleanup script outside this repository. Its commands are not an approved deletion policy. Product decisions are recorded in shared project notes. No measured product scan results or shipped interface exist yet.

## Product Principles

- Explain consumption before suggesting action.
- Keep unknowns and evidence limits visible.
- Make ownership and removal consequences understandable.
- Preserve user data and trust through explicit, scoped actions.
- Prefer a small complete system over speculative flexibility.

## Open Decisions

The latest warm CleanMyMac-inspired direction supersedes the earlier Space Layers aesthetic. Design specifications live in [docs/design/index.md](docs/design/index.md). Exact visual execution is being implemented and reviewed. Planned capabilities are not claims about what the current scaffold already supports.
