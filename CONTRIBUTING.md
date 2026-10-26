# Contributing

Small, focused contributions are welcome: clearer documentation, reproducible bug
reports and improvements to the existing scaffold. Follow the
[code of conduct](CODE_OF_CONDUCT.md).

## Understand the scope

Start at the [documentation index](docs/index.md), then read the
[development workflow](docs/development/workflow.md) and relevant specifications.
The app currently shows desktop metadata only. Scanning, artifact recognition and
cleanup are [planned product work](docs/product.md), not existing functionality.
Propose scope and safety requirements before implementing those capabilities;
do not infer deletion rules from examples or design proposals.

## Develop and validate

1. Branch from an up-to-date `dev` into a descriptive feature branch. In an assigned
   worktree, use its assigned branch and preserve other contributors' edits.
2. Follow [local setup](docs/development/local-setup.md): install prerequisites,
   run `npm ci`, then `npm run dev` from the repository root.
3. Record intended behavior and acceptance before behavior changes, using an
   existing specification or the [specification template](docs/specs/template.md).
   A trivial documentation task can serve as its own specification.
4. Keep edits focused in the owning layer. Keep Rust domain logic independent of
   React and Tauri; validate privileged operations at the Rust boundary.
5. For application edits, run `npm run check` and focused tests for changed behavior.
   Use `npm run format` when formatting needs correction. For documentation-only
   edits, check links, referenced paths, template formats and `git diff --check`;
   no application rebuild is needed. Report skipped checks and evidence limits.
6. Commit your edits and open a pull request targeting `dev`, describing behavior,
   validation and safety implications. Keep unrelated changes separate. Do not
   merge into `dev` or `main` or publish releases as part of a contribution.

Use npm and preserve both `package-lock.json` and `Cargo.lock`. Keep repository
prose English. Preserve the [MIT license](LICENSE) and existing attribution.

## Reports and safety

For bugs, include expected versus actual behavior, minimal reproduction steps,
platform/tool versions and sanitized diagnostics. For features, describe the user
problem, proposed outcome and relevant risks. Distinguish current behavior from
future product ideas.

Omit sensitive file paths, file contents, credentials and personal information
from issues, pull requests, logs and screenshots. Use synthetic paths and isolated
fixtures; never test destructive cleanup against real user data. Any future
cleanup change needs explicit scope, consequences, partial-failure handling and
evidence for recovery claims. No finalized deletion policy or security audit is
claimed.

Read [SECURITY.md](SECURITY.md) before reporting vulnerabilities. Do not put
sensitive vulnerability details in public issues.
