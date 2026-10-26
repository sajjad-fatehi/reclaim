# Development artifacts, projects and worktrees

Follow [shared requirements](index.md). Consume the actual scanner identity/measurement contract and publish the actual typed finding/ownership/consequence/eligibility contract for Docker, Development views and cleanup consumers.

## Required coverage

Implement evidence-based read-only recognition for these initial families. A recognizer must have representative fixtures and explicit non-matches; a string equal to a folder name is insufficient ownership evidence.

| Family | Initial inventory and explanation |
| --- | --- |
| JavaScript projects | node_modules; Next/Turbo/Nuxt/SvelteKit outputs; project manifests and source relationship |
| Python projects | .venv/venv with environment metadata; project ownership; distinguish user source from generated environment |
| Build/log data | Rust target and verified framework outputs; project logs when evidence supports the classification, not every .log file |
| Package stores | npm, pnpm, Yarn, Bun, pip and uv stores/caches; distinguish shared stores from per-project dependencies |
| Runtimes/toolchains | nvm Node versions, Go module/build caches, Cargo registry/cache and Homebrew cache inventory; never call installed runtimes removable merely because old |
| Editors/coding tools | VS Code, Cursor, JetBrains and identifiable Claude/Codex worktrees/artifacts; separate cache/logs from workspace history, chats, credentials, settings and unsaved state |
| Git worktrees | Standard Git registry plus Claude/Codex worktree locations verified as actual worktrees; associated repository, branch, main/linked status and changes |
| Temporary data | User-selected temporary locations including the macOS /tmp alias; inventory and contextual explanation, no blanket cleanup or age-based safety claim |

## Behavior and safety

- Discover configured tool locations with supported bounded mechanisms when available; do not assume every machine uses default paths. Do not invoke install/update/prune/cleanup commands during detection or auto-start missing tools.
- Recognize project boundaries from verified metadata. Assign native identities and provenance/confidence, distinguish shared/project-owned storage, deduplicate nested/linked findings and avoid counting parent and recognized child again in totals.
- Determine worktree change status without mutating Git. Untracked/ignored/unknown state is not clean; missing tools, corrupt metadata, detached or locked worktrees remain inspectable with uncertainty. Do not expose file contents or execute repository hooks.
- Explain regeneration/download cost and why a finding deserves inspection. Age is a hint only. Recognition and cleanup support are separate fields; unsupported/uncertain cases remain inspection-only.
- Missing tools/paths and malformed editor/workspace/package metadata produce scoped unavailable/unknown results, not application failure or manufactured zero size. Do not scan outside explicitly authorized roots to find global caches.

## Acceptance and proof

- Every family above has fixture-backed positive/negative behavior and explicit unsupported/configuration limits documented in a coverage matrix; no family may be silently omitted.
- Shared stores and nested artifacts are counted once; false-positive user folders, project source, credentials and persistent editor history never receive a cleanup-safe label.
- Clean/dirty/untracked/ignored/locked/detached worktree fixtures and malformed/missing-tool cases pass focused tests. Native methods operate read-only and remain bounded/cancellable with the scan session.
- Publish concrete contracts and actionable consequence explanations; required checks pass. No cleanup methods are executed in this task.
