# Information architecture

[Design index](index.md)

## Global shell

Persistent left navigation: Overview, Explore, Development, Review & Clean. Settings lives at the bottom, outside the primary task list. A window toolbar identifies the selected volume and scan state. Keep desktop window controls, drag regions and interactive hit areas separate.

One scan session owns coverage and results across sections. Each section presents a view of the same findings, not an independent scan or a second copy of counted storage. Preserve volume, filters, selection and scroll position when navigating away and back.

Scan state remains visible after leaving Overview. Review selection is session state and survives navigation, but must be revalidated before any removal. Persistence across app restarts is not specified for the first version.

## Overview

First question: where did the space go?

Reading order:

1. Volume identity, capacity, used and free space, measurement time.
2. Coverage status and scope; incomplete access is visible beside the numbers.
3. A segmented usage strip and matching ranked category list.
4. A small set of evidence-backed investigation suggestions.

Distinguish whole-volume measurements from measured scan scope. Space outside the scanned scope is not a detected artifact. Unexplained usage stays explicit; do not force filesystem scan totals to equal volume usage by inventing a category measurement.

Suggestions open relevant filtered findings. Their first action is inspection, not deletion. Before any scan, replace usage and recommendations with the scan-location setup; never fill the screen with fake zeros.

## Explore

First question: which exact paths account for this usage?

Breadcrumbs and scoped search sit above a sortable folder/file list. Columns: name, measured allocated size when available, recognized owner/type and measurement status. Explain the chosen size metric and expose logical size separately when useful. Hidden entries remain discoverable. Label filtering/exclusion so hidden results cannot look like missing storage.

Selecting a row opens its inspector; opening a directory navigates into it. Provide distinct actions and keyboard semantics. A chart selection filters or focuses its matching rows and can always be cleared. Unknown paths remain explorable without a recognizer.

## Development

First question: which projects and tools own the consumption?

Projects and Tools are local tabs, not extra top-level pages.

- Projects: source, dependencies, build output, logs and related worktrees for each project.
- Tools: shared stores, runtimes, container resources and editor/coding-tool data.

Show a shared artifact once in accounting and link its related projects. Recognizer uncertainty is visible. Worktrees with uncommitted changes and persistent container data receive specific restrictions, never a generic safe-to-clean badge.

## Review & Clean

First question: what exactly will happen to the selected items?

Show selected findings grouped by cleanup method and consequence. Provide paths, measurement timestamp, size basis, evidence, rebuild/download cost, data warnings and an individual remove-from-selection action. Grouping is organizational; it does not imply shared permission to delete.

Use Estimated recovery only when defensible; otherwise say Recovery unknown. Keep operation history below current review or in a local History tab. No separate history section initially.

## Shared inspector

Sections: identity and path; ownership and evidence; size and measurement basis; scan/permission status; consequences and supported actions. Long paths support wrapping, selection and copying; never hide the full path behind hover alone.

Actions may include Show in Finder, Copy path, Inspect related project, and Add to review when supported. Selection does not grant deletion permission. Unknown ownership does not automatically enable cleanup.

## Settings

Initial groups: Scan locations, Exclusions, Access, Appearance. Appearance defaults to System. Explain exclusions' effect on coverage. Access describes why a particular operation needs permission and offers the applicable macOS route without implying the app can bypass system restrictions.
