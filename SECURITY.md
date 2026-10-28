# Security

## Project status

Reclaim is an early desktop application, not a released cleanup tool. It grants
application metadata and narrow, user-started read-only scanner commands only to
the local main window. Native folder selection or explicitly registered native
candidates grant process-local scopes; frontend paths cannot grant authority.
Traversal uses no-follow directory handles and does not read file contents, cross
mount boundaries, request elevated privileges or implement cleanup. Generic
filesystem, shell and frontend dialog permissions remain unavailable. Scans are
not atomic filesystem snapshots; concurrent changes and inaccessible entries
produce qualified coverage. See [scanner contract](docs/specs/features/scan-foundation.md).
After measurement, scoped development recognition reads bounded known manifests,
configuration and cache markers. Optional Git status uses installed CLT Git under
macOS read/write/network/exec confinement, never hooks or missing-tool installers.
It may read repository contents to determine change status, but returns no contents
or raw command output. Recognition is inspection-only, not deletion authority.
See [recognition boundaries](docs/specs/features/development-recognition.md).
Full Disk Access guidance adds two argument-free commands, `access_status` (read-only
directory probes of TCC-protected locations, no names or contents read or logged) and
`access_open_settings` (opens only the fixed Privacy & Security → Full Disk Access pane).
Reclaim never requests elevation or bypasses macOS privacy controls; the user grants
access, and unsigned or development builds attach the grant to the launching binary or
terminal and may lose it across rebuilds. See
[access guidance](docs/specs/features/overview-and-scope.md#full-disk-access-guidance).
There is no supported release line or promised security maintenance window. No
security audit or finalized deletion policy is claimed.

## Reporting a vulnerability

As checked on October 26, 2026, GitHub private vulnerability reporting is disabled
for this repository. No other private reporting route has been verified. This
repository therefore cannot currently offer a verified private vulnerability
submission channel. No response time or support commitment is promised.

Do not publish sensitive vulnerability details, exploit instructions, credentials,
private file paths or file contents in public issues, pull requests or comments.
The repository's [Security page](https://github.com/sajjad-fatehi/reclaim/security)
may show a **Report a vulnerability** option if private reporting is enabled later;
use it only if available. Otherwise, retain sensitive details privately. If useful,
open a public issue requesting a private reporting channel without disclosing the
vulnerability or affected private data.

Use synthetic paths and disposable fixtures for any reproduction. Never run
destructive cleanup against real user data.
