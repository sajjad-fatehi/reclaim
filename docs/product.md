# Product direction

## Status and purpose

This document records planned product behavior, not implemented capabilities. The
current foundation contains product and design documentation; a minimal application
scaffold does not establish scanning, recognition or cleanup support.

Reclaim helps developers and people using coding tools, including those unfamiliar
with generated development artifacts, answer three questions:

- Where did disk space go?
- What deserves inspection?
- What are the consequences of removing it?

Initial delivery targets macOS. Windows and Linux remain future directions, not
supported platforms. Product and repository documentation initially use English.
See [product context](../PRODUCT.md) for confirmed purpose and constraints and
[design specifications](design/index.md) for interface direction and proposals.

## Overview

Show volume capacity, used space and free space alongside meaningful categories:
projects, development tools, personal files and unidentified usage. Distinguish
whole-volume measurements from measured scan scope. Unknown consumption remains
visible rather than being assigned an invented explanation.

Expose scan locations, coverage, exclusions and inaccessible paths beside results.
Prioritize investigation recommendations with reasons and supporting evidence.
Provide estimated reclaimable space only when evidence supports it; otherwise
identify recovery as unknown. A measured folder size alone is insufficient.

## Explore

Navigate folder sizes, including hidden entries, with search, sorting and reveal
in Finder. Keep unrecognized usage inspectable without requiring an artifact
recognizer. Explain size measurements and access limits so missing or excluded
results cannot appear to be an empty folder or a complete scan.

## Development

Provide connected views by project and by tool:

- **Projects:** source, dependencies, build output, logs and related worktrees.
- **Tools:** shared package stores, runtimes, container resources, editors and
  coding-tool data, with links to related projects where ownership is known.

Distinguish project-owned storage from shared storage. Count shared artifacts once
in totals, even when several projects or views reference them; uncertain ownership
stays explicit. npm, pnpm, Python, Docker, editors and coding tools are candidate
ecosystems, not guaranteed first-release integrations.

## Review & Clean

Before removal, show selected items' exact paths, measured sizes and size basis,
recommendation rationale, evidence and consequences. Distinguish rebuildable
artifacts, personal data and unknown state; explain rebuild or download costs.
Selection or recognition alone does not establish deletion safety.

Protect or explicitly surface uncommitted changes in worktrees. Distinguish
persistent Docker data from rebuildable container artifacts; never present it as
generically safe to remove. Limited cleanup must account for these risks before
offering an operation.

Report success or failure per operation and actual recovered space where it can
be measured. Offer recovery options only when the operation supports them. Keep
initial operation history within Review & Clean, not a fifth primary section.
Never promise universal undo or guaranteed deletion safety.

## Shared item details and settings

Across sections, item details explain identity and path, owning tool or project,
size and measurement basis, why it deserves inspection, removal cost and
consequences, confidence, and evidence or access limits. Explain why an item is
considered large only when evidence supports that judgment. One scan cannot
establish growth history.

Minimal settings cover scan locations, exclusions and permissions. Suggested
locations are reviewed and editable before the user explicitly starts scanning.
Explain how exclusions and permissions affect coverage; do not imply access can
bypass operating-system restrictions.

## Initial delivery and invariants

Deliver in this order:

1. Read-only scanning and space exploration.
2. A small, high-value set of development-artifact recognizers, prioritized from
   actual usage rather than an exhaustive integration checklist.
3. Limited, explainable cleanup with explicit consequences and operation results.

These remain planned stages, not claims about the current scaffold. Throughout:

- Old does not mean unused or safe to delete.
- Folder size does not guarantee physically reclaimable space. APFS sharing and
  snapshots can change the relationship; expose those accounting limits.
- Incomplete scans, inaccessible paths, exclusions and unknown usage remain visible.
- Recovery and safety claims must stay within available evidence and supported
  operations.

## Deferred work and open decisions

Duplicate detection, app uninstalling, RAM cleaning and automatic cleanup are
deferred. Initial artifact priorities and technical feasibility of measurement,
recognition, cleanup and recovery integrations remain open decisions. Resolve
them from observed usage and technical evidence without treating candidate tools
or proposed designs as delivery commitments.
