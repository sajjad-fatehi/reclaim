# Interaction flows and states

[Design index](index.md)

## First use and scan

1. Explain the job in one sentence: Find what is using your disk space.
2. List suggested scan locations with path, purpose and access state. Allow removal and adding locations through an appropriate system picker.
3. Explain coverage limits. Request additional access in context, rather than gating the entire application on broad permissions.
4. The user selects Start scan. Do not scan automatically during onboarding.
5. Show the current stage and location, accumulated measured results, elapsed time and Cancel scan. Use determinate progress only if the denominator is known.
6. On completion, show scope, time and unresolved access/errors. Open Overview with Explore available immediately.

Cancellation preserves completed measurements as partial results and labels them accordingly. Failure preserves usable findings when possible, explains the affected scope, and offers a scoped retry. No-results-found differs from scan-not-started and from no-readable-locations.

Proposed lifecycle: Not started → Scanning → Completed / Partial / Cancelled / Failed. Partial findings may appear while scanning; a location-level error need not mean the whole scan failed. If a rescan is supported, retain the prior snapshot until replacement data is available and distinguish the two.

## Investigation

Select a category or suggestion → matching list → selected finding → inspector → related project/tool or review selection. Provide breadcrumbs and predictable back behavior. Changing filters never silently broadens review selection.

Do not reorder rows continuously while the user is interacting. Offer an explicit refresh/order update when new measurements change ranking. Announce progress summaries at a restrained cadence, not one screen-reader message per file.

## Review and cleanup

1. Add only supported findings to review; unsupported items retain inspection actions.
2. Explain the intended cleanup method and consequence for each item. A supported tool-owned prune operation may differ from deleting a directory.
3. Immediately before execution, revalidate identity, permissions, relevant activity and safety conditions. A renamed, changed or missing target does not inherit stale consent; mark it changed and require a new review where needed.
4. Present a confirmation naming the affected selection and whether the operation is reversible. Initial focus is Cancel; Enter must not accidentally confirm an irreversible operation.
5. During execution, disable repeat submission and report item-level progress. Cancellation stops future work where supported; it does not promise to undo completed work or interrupt an unsafe-to-interrupt operation.
6. Report Removed, Skipped, Failed or Changed for each item, with reason and available next step. Retry only failures still eligible after revalidation.

Record measured free-space change when available, with its limitations. Concurrent disk activity means the difference is not automatically attributable to this cleanup. Do not relabel selected folder sizes as recovered space. Recovery controls appear only for operations with a real recovery mechanism.

Eligibility review and local history are implemented as a partial delivery. All
removal methods remain unavailable: selection opens explanations, never an
executable cleanup plan. Confirmation, actual removal, execution cancellation and
recovery remain unmet requirements. Verification never executes destructive
commands against the user's real files.

## State matrix

| State | Visible information | Available action |
| --- | --- | --- |
| Not started | Scope suggestions; no result numbers | Edit scope; Start scan |
| Scanning | Stage, current scope, provisional measurements | Cancel; inspect completed findings |
| Permission denied | Exact affected scope; coverage impact | Continue with partial coverage; manage access |
| Partial/cancelled | Completed scope and missing scope | Inspect; retry appropriate scope |
| No matching results | Current filter and scope | Clear filters; edit scope |
| Unknown artifact | Path and measured facts; recognition unknown | Inspect; reveal/copy path |
| Stale result | Measurement time; relevant changes known | Rescan or revalidate |
| Cleanup failed | Item, reason, completed operations | Inspect; eligible scoped retry |
| Measurement unavailable | Metric unavailable, not zero | Inspect explanation; retry if applicable |

## Copy and accessibility

Use factual English: Not scanned, Access required, Ownership unknown, Measured size, Add to review. Avoid Junk, Guaranteed safe, Optimize now and a device health score.

All flows work with keyboard and VoiceOver. Restore focus to the triggering control after an inspector/dialog closes. Keep status messages associated with the relevant region. Never rely solely on color, hover, icon shape or animation to communicate consequences.
