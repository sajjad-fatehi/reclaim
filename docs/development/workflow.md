# Development workflow

Specification-driven development means recording requirements and acceptance before implementation. Scale the record to the change's risk; no special tool or routine approval ceremony is required.

## From request to evidence

1. **Understand.** Read applicable agent instructions, the task, relevant code and specifications. Trace existing behavior and callers, identify the owning layer, and check what users or data could be affected.
2. **Specify.** Before changing behavior, record intended observable behavior, scope and non-goals, acceptance criteria, relevant edge/error cases, safety constraints and genuine unresolved decisions. Update an existing specification first; otherwise use the [lightweight template](../specs/template.md). A precise durable specification path or shared project note ID is sufficient in a board task; do not copy the specification repeatedly. A trivial edit can use its task description as the entire specification.
3. **Resolve blockers.** Ask about unresolved product, security or data decisions that prevent correct work. Choose ordinary technical details within existing conventions. If evidence contradicts an assumption, report it and correct the affected plan/specification before proceeding; do not silently redefine product intent.
4. **Implement.** Make the smallest complete change in the responsible layer. Update affected callers, tests, fixtures, configuration and exports. Preserve unrelated edits and behavior.
5. **Verify.** Match acceptance criteria to the smallest meaningful checks. Test non-trivial behavior and relevant failure cases. Reuse passing evidence for unchanged work; fix failures and rerun only affected checks. Do not expand into unrelated cleanup or testing after acceptance passes.
6. **Document and report.** Update the authoritative specification whenever behavior changes, including any accepted scope adjustment. Update affected user/developer guidance and [documentation navigation](../index.md). Report changed files, exact verification commands and outcomes, evidence limits and remaining blockers. Commit on the assigned branch; stop when acceptance is met.

## Ownership and boundaries

The approved stack is Rust, Tauri 2, React, TypeScript and Vite, initially targeting macOS. The current application is a metadata-only desktop scaffold. The following boundaries govern future behavior; see [local development](local-setup.md) for existing directories and commands.

- **Rust domain core:** Own domain rules and accounting invariants independently of the UI and Tauri adapter. Keep core logic testable without a WebView or Tauri runtime.
- **Tauri adapter:** Own transport, application wiring and permission enforcement. Expose only operations the task requires. Validate frontend input, paths and authorization at the privileged boundary; frontend checks cannot grant filesystem access. Return useful failures without exposing private data unnecessarily.
- **React/TypeScript frontend:** Own presentation and interaction state. Use typed boundary contracts; do not duplicate core rules or bypass the adapter with arbitrary filesystem or shell access. Preserve keyboard access, focus behavior, readable errors, semantic controls and relevant assistive-technology support.
- **Shared changes:** Extend existing contracts and helpers rather than creating competing implementations. Keep dependencies minimal and justified. Do not introduce scanner or cleanup capabilities through a foundation/documentation task.

## Safety and verification

- Treat paths and other external input as untrusted. Limit privileged operations to validated, authorized scope; avoid broad capabilities or arbitrary command execution.
- Protect uncommitted worktree changes and persistent user/container data. Never run destructive cleanup on real user data as verification. Use disposable, isolated fixtures and verify that out-of-scope fixture data survives.
- Do not log private file contents, credentials or unnecessary personal paths. Preserve existing license and human attribution.
- For cleanup behavior, specify deletion consequences, partial failure handling and any supported recovery before implementation. Never promise blanket undo or equate directory size with physically recoverable space without evidence.
- Select checks from actual repository tooling. Documentation-only edits need link/path checks and a content review, not an application rebuild or new dependencies. For application changes, choose focused tests and required project gates based on the delivered scripts/configuration; do not invent runnable commands.
- Distinguish local verification from hosted CI and from native macOS/WebView evidence. Mark checks not run and unresolved risks explicitly. No automatic publishing or unrequested production operations.

## Branch and delivery flow

Start from manager-prepared `dev` in the assigned isolated worktree. Commit to the assigned task branch and report its name and commit. PRs target `dev`; the manager coordinates prerequisite availability, PR creation and integration. Do not merge into `dev` or `main`, publish releases or modify repository settings without separate authorization.

## Written workflow walkthroughs

These examples check the workflow's proportionality; they do not authorize application features or claim runtime test results.

### Trivial documentation edit

Task: correct a misspelled heading without changing meaning. The task itself records scope and acceptance: spelling corrected, existing links still resolve, no behavior change. Read the affected document and incoming links, edit the heading and any affected anchors, then check those links and the diff. Report those checks and commit. No separate specification file, application build or approval ceremony is needed.

### Future cleanup change with data-loss risk

Before implementation, record selected paths, ownership evidence, allowed removal, protected worktree/persistent data, explicit user authorization, cancellation, permission errors, concurrent path changes and partial failure outcomes. Acceptance includes preserving protected and out-of-scope data and reporting each operation's result; recovery is promised only if supported. Resolve any unclear deletion policy before coding. Keep validation and destructive operations in the responsible Rust boundary, then verify success and failure with isolated disposable fixtures, including protected and out-of-scope fixture sentinels. Never use real user data. Update the specification with accepted behavior and report exact checks plus any unverified platform behavior. This walkthrough grants no permission to implement or run cleanup now.
