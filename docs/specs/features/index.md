# Application feature delivery

Status: authorized implementation specification. The user requested all approved app features, their specifications and execution. These documents describe acceptance; they do not claim the scaffold already implements it.

Product authority: [product direction](../../product.md), [design index](../../design/index.md), [interaction flows](../../design/interaction-flows.md), [icons and motion](../../design/icons-and-motion.md). Follow [SDD](../../development/workflow.md).

## Complete delivery map

| Deliverable | Specification | Prerequisite outputs |
| --- | --- | --- |
| Read-only scanner, scope authority and typed native contract | [Scan foundation](scan-foundation.md) | Existing scaffold |
| App shell, design primitives, appearance and navigation | [Application shell](application-shell.md) | Existing scaffold |
| First run, real scan controls, Overview and scan settings | [Overview and scope](overview-and-scope.md) | Scanner + shell |
| Filesystem exploration and shared inspector | [Explore](explore.md) | Scanner + shell |
| Project, filesystem-artifact and worktree recognition | [Development recognition](development-recognition.md) | Scanner contract |
| Optional local Docker inventory | [Docker](docker.md) | Scanner + recognition contract |
| Connected Projects/Tools views | [Development views](development-views.md) | Recognition + Docker + shell |
| Explicit safe removal, Review & Clean and operation history | [Review and cleanup](review-and-cleanup.md) | Overview + Explore + Development views and their contracts |

Every approved primary section, item details, initial settings, access/coverage state, history, English copy, light/dark appearance, icons and motion has an owner above. No separate speculative polish or generic test phase substitutes for acceptance within each feature.

## Shared invariants

- macOS first; React/TypeScript presentation, Rust domain rules and narrow validated Tauri transport. Reuse installed/platform capabilities; justify new dependencies by real behavior. Preserve MIT/human attribution and dev workflow.
- All scanning is read-only and user-started. Suggested paths are editable before starting. No automatic home/disk traversal on launch, hidden background agent, telemetry, hosted service or private-data upload.
- Filesystem identity, ownership, scan generation and measurement timestamps live in authoritative native state. Frontend text, paths, sizes, flags or selected identifiers cannot grant native permissions or prove cleanup eligibility.
- Treat paths, external metadata, tool output and persisted configuration as untrusted. Do not follow symlinks to escape scope; detect directory cycles, overlapping roots, hard links and concurrent path changes. No root privileges, Full Disk Access bypass or arbitrary shell command endpoint.
- Show logical size versus allocated accounting basis, incomplete coverage, unknown ownership, stale data and unavailable metrics. Count shared storage once; do not equate apparent size with recoverable physical space. APFS clones, snapshots and unrelated volume usage remain qualified unknowns.
- Never infer inactivity or safe deletion from modification time alone. Distinguish user/source data, credentials, persistent container data and regenerable artifacts. Do not inspect file contents unnecessarily or emit private paths/content into logs.
- All core behavior includes focused success/error/edge tests. Destructive tests use disposable fixtures and out-of-scope sentinels only. No worker may validate deletion on the user's files or run real Docker cleanup as a test.
- UI implements real data and all relevant not-started/loading/partial/cancelled/empty/error/stale states. Synthetic data is confined to tests or explicitly labelled development fixtures. Preserve keyboard, focus, VoiceOver semantics, reduced motion and usable text enlargement.

## Contracts and dependencies

There are currently no scanner, artifact or cleanup APIs to reuse. The scan producer owns discovering the simplest fitting implementation and documenting its actual typed transport/session/scope/identity/error contract before consumers start. The recognition producer similarly publishes its concrete findings/ownership/eligibility contract. Consumers extend those outputs, not independently invented competing models.

The shell owns navigation, theme and reusable presentation primitives, not scan/domain data. The Overview feature owns shared frontend scan-session orchestration. Explore and recognition may use the scan contract independently; integrate session ownership through the existing producer rather than duplicating live scans. The cleanup feature owns review selection across completed views and the only mutation boundary.

Dependencies must be Done, integrated into dev and present in each consumer checkout before submission. Shared files alone are not prerequisites. The manager uses submission planning for overlap and records reviewed producer commits. Workers report material contract deviations instead of silently widening scope.

## Out of scope

Duplicate detection, app uninstallation, RAM cleaning, automatic/scheduled deletion, Windows/Linux delivery, sync/accounts, cloud scanning, privileged helpers, app-store/notarized distribution and speculative plugins remain deferred. Explicit inspection-only safety cases are supported outcomes, not permission to claim unsupported cleanup methods exist.

## Completion gate

Each task supplies its specification-to-evidence mapping, focused passing checks, screenshots for visible results, commit and material limits. Required CI runs must pass. Do not rerun unchanged builds/reviews merely for a new task message. Verify newly introduced interactions at integration boundaries. Stop after acceptance; do not invent optional work.
