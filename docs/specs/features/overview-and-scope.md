# First run, scan settings and Overview

Follow [shared requirements](index.md) and [interaction flows](../../design/interaction-flows.md). Consume the delivered scanner and shell contracts; do not invent parallel ones.

## Required behavior

- First run shows editable suggested locations, purpose and access state. Add locations through the actual native picker, remove suggestions and explicitly Start scan. No traversal before that action.
- Settings manages scan locations, exclusions and access explanations; use the shell's appearance settings. Persist preferences locally with validated atomic writes and safe corrupt-file handling. Persisting a path does not grant fresh consent to scan on launch.
- Exclusions have clear exact/subtree semantics and visible coverage effects. Handle duplicate, nested, missing and inaccessible roots, new/removed volumes and changed permissions. Apply scope changes to the next scan; do not silently alter an active scan.
- Own one frontend scan-session orchestration layer consumed by completed sections. Surface stage/location/elapsed time, measured findings, cancellation and completed/partial/failed states. Do not invent percent completion when total work is unknown or continuously reorder interacting lists.
- Overview shows real volume totals, measurement time, scope coverage, segmented usage and matching ranked categories. Keep unscanned/unexplained/unavailable values explicit; no fake reconciled totals or inflated reclaim estimates.
- Category selection and reasoned investigation suggestions open corresponding Explore/Development context when available. Use the signature follow-the-space relationship and accessible textual equivalents.
- Refresh results as a deliberate scan generation; label retained older data and handle volume/disconnection/error changes without losing usable partial results.

## Acceptance and proof

- Native end-to-end fixture flow: suggested scope → edit/exclude → explicit start → progressive results → Overview; restart preserves settings but performs no auto-scan.
- Cancel, no readable roots, no matches, partial permission failure, corrupt settings and stale generation each render distinct actionable states.
- Fixture categories and volume/scope numbers are truthful and do not double-count. An unknown metric is not displayed as zero.
- Focused Rust preference and React session/state tests, relevant repository checks and a native UI smoke pass. Show actual fixture screenshots; never scan the user's private folders to populate a showcase.
- All copy is English; keyboard/VoiceOver/status cadence, both themes and reduced motion meet the shared UI contract.
