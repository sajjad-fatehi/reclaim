# Explore and shared item inspector

Follow [shared requirements](index.md), [information architecture](../../design/information-architecture.md) and the delivered scanner/shell. Reuse Overview's session orchestration when integrated; do not start a separate background scan.

## Required behavior

- Browse real scan findings with breadcrumbs, scoped search, stable sorting and folder/file rows including hidden and unrecognized entries. Directory navigation and row inspection are distinct actions.
- Show measured size/basis, name, ownership/type when known and measurement status. Allow clear filter reset/back behavior, preserve selection/scroll through navigation and expose full identities for equal names in different directories.
- Support large result sets through bounded retrieval and a measured accessible rendering strategy. Deep paths, Unicode, lossless native identities and invalid-display bytes must not break navigation or escaping.
- Implement the shared inspector: path/copy, owner/evidence, size/time/basis, coverage, uncertainty, removal consequences and related-project/tool links. Unknown data stays unknown. Do not infer growth history from one scan.
- Show in Finder and Copy path are explicit user actions. The native reveal endpoint accepts an authorized finding identity, revalidates its path and uses a safe platform API without shell interpolation or an arbitrary program/URL opener.
- Reflect pending/partial/cancelled/stale/denied/vanished findings. Safe inspection remains possible when cleanup is unsupported. Add-to-review is supplied by the later cleanup task, not a fake button now.

## Acceptance and proof

- Fixture tests cover navigation, scoped searching, ascending/descending sorting, duplicate names, empty filters, long paths, partial measurements and stable selection under updates.
- Forged IDs and paths outside the scan scope cannot trigger native reveal. A disappeared target yields a useful error. No private contents or raw error dump is rendered.
- Native fixture interaction and keyboard/VoiceOver smoke evidence cover table, inspector and Finder handoff; both theme and compact/text-enlarged layout remain usable.
- Relevant focused tests and required checks pass; no destructive operation or duplicated scan-session store is introduced.
