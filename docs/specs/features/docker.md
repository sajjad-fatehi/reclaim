# Optional local Docker inventory

Follow [shared requirements](index.md) and the delivered recognition contract. Docker is an optional local tool, not a prerequisite to launch Reclaim.

## Required behavior

- Detect a supported installed Docker CLI and an available local daemon without starting Docker, pulling images or changing context. Remote contexts/endpoints are inspection-unavailable for this local-disk product; do not silently contact remote hosts.
- Use bounded typed commands with arguments, timeouts, cancellation and validated structured output; never interpolate a shell command from user input. Do not collect container secrets, environment variables or file contents.
- Report available local image/container/build-cache/volume inventory and ownership/reference state, qualifying shared layers, active use and measurement uncertainty. Report unavailable per-item metrics instead of allocating an aggregate arbitrarily.
- Distinguish persistent volumes and container writable layers from rebuildable images/caches. Mark persistent, referenced, running, ambiguous and unsupported resources as inspection-only.
- Attach validated identities and evidence to the common artifact model. Do not add Docker's virtual-disk apparent size to its internal layer totals or to counted filesystem parents; show the relationship and keep overlap explicit.
- Unsupported versions/alternative engines, malformed output, command timeout, absent CLI, stopped daemon and remote context each produce an honest state with remediation. Alternative engines may be represented as unsupported; no unverified compatibility claim.

## Acceptance and proof

- Stubbed process/transport fixtures cover success, shared layers, active/persistent resources, malformed/partial output, unavailable tool, stopped daemon, remote context and timeout/cancel.
- No engine startup, network fetch, container mutation or privileged generic shell endpoint is introduced. Persisted context is not changed.
- Document which metrics are exact, approximate, shared or unavailable and which supported Docker versions/formats were verified. Deliver real read-only inventory, not a static integration tile.
- Relevant checks pass and Development consumers receive the actual contract. Real Docker deletion is never used as verification.
