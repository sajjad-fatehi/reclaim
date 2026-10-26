# Desktop layouts and components

[Design index](index.md)

## Window model

Proposed reference frames: 1440 × 900 and 1180 × 760 CSS pixels; verify compact behavior at 900 × 600. These are design targets, not enforced native window minima. This is a desktop app; no phone navigation or marketing-page breakpoint scheme is specified.

Use a 208-pixel initial navigation rail, a flexible work area and an optional 320–360-pixel inspector. Keep primary content flexible rather than fixing a full grid to one screenshot size. At compact widths the inspector becomes an accessible sheet; do not compress path and size columns into unreadable slivers. Preserve navigation labels or offer an explicitly labelled collapse control.

Window toolbar target: 48–56 pixels with adequate macOS titlebar clearance. Data toolbar: approximately 44 pixels. Standard result rows: 40 pixels; compact density is not an initial user setting. These targets require rendered validation, text scaling and actual window-control testing.

## Overview composition

Navigation rail → main area with volume/scan header → full-width usage strip → ranked category rows and investigation suggestions. An inspector occupies the right only after selection. The usage strip is a proportion display with a matching textual list, not a giant decorative disk illustration.

The first viewport should expose the selected volume, coverage limits, leading measured consumption and the next investigation action. Avoid four equally prominent metric cards competing with the actual disk accounting.

## Explore composition

Navigation rail → breadcrumbs and scoped search → sortable result table → inspector. Filters remain directly above the results they affect. A selection summary is attached to the list and links to Review & Clean; it does not conceal row data or become a permanent Delete button.

Keep size figures right-aligned, filenames left-aligned and status labels in consistent columns. Preserve a stable primary column and expose secondary columns through details when space is constrained. Retain full-path access and deliberate horizontal scrolling where necessary.

## Development composition

Navigation rail → Projects / Tools tabs → scoped filters → comparable result rows → inspector. Rows use the same size, status and ownership grammar as Explore. Switching views changes grouping, not underlying counted bytes.

## Review composition

Navigation rail → selection summary and coverage/measurement caveat → consequence-grouped items → action footer. Keep the primary action close to the selection and use specific copy such as Review removal. The final irreversible action belongs to a confirmation with item details and consequence, not the general navigation bar.

## Required component inventory

| Component | Responsibility and states |
| --- | --- |
| App shell | Navigation, volume context, titlebar-safe controls |
| Scan scope editor | Suggested/manual paths, access status, exclusions |
| Scan status region | Not started, scanning, partial, completed, error |
| Usage strip and legend | Measured proportions, selected segment, unavailable scope |
| Result table | Sort, selection, loading, empty, partial, error |
| Breadcrumbs and search | Scoped navigation, clear action, no matches |
| Ownership/status label | Evidence-qualified, textual, never color-only |
| Item inspector | Facts, provenance, uncertainty, consequences, actions |
| Review selection | Per-item eligibility and changed/stale state |
| Confirmation dialog | Specific consequences, cancel-first focus |
| Operation result list | Per-item outcome and supported recovery |
| Inline notice | Scoped explanation and next action |

Reuse platform-aware semantic controls before inventing custom ones. Do not select a component library solely to fill this inventory.

## Data stress cases

Validate a single finding and large result sets without rendering all rows at once. Choose pagination or accessible virtualization after measurement; do not adopt a library from a generic row-count threshold. Stable identifiers preserve selection and focus when data updates.

Test deep paths, Unicode names, spaces, names over 200 characters, equal filenames in different directories, unavailable sizes and permission errors. No shortened label may be the only identity available before an action.

Illustrative fixture only, not observed data: capacity 512 GiB; used 384 GiB; free 128 GiB. An explicitly complete synthetic breakdown may show projects 142 GiB, shared tools 126 GiB, personal files 76 GiB and unexplained 40 GiB. Its totals reconcile by construction; real partial scans must never imply that same completeness.
