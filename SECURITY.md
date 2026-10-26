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
