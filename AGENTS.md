# Coding-agent rules

- Before editing, read applicable instructions, the task, relevant specifications and existing code. Start documentation discovery at [docs/index.md](docs/index.md).
- Follow [the specification-driven workflow](docs/development/workflow.md). Record acceptance before behavior changes; a trivial task description can be the whole specification.
- Extend the layer that owns the behavior. Reuse existing patterns; avoid duplicate behavior, speculative abstractions and unrelated refactors.
- Use descriptive names and clear ownership. Explicitly type exported APIs and trust boundaries. Do not use TypeScript `any` or unsafe casts to bypass validation or type errors.
- Handle errors first with guard clauses; keep nesting shallow. Follow existing formatting. Add dependencies only with a concrete justification. Comment non-obvious rationale, not mechanics.
- Keep Rust domain logic independent of React and Tauri. Put privileged operations behind narrow, validated Rust interfaces; follow [boundary guidance](docs/development/workflow.md#ownership-and-boundaries).
- Preserve security, privacy, accessibility and data-loss protections. Never verify destructive cleanup against real user data; use isolated fixtures. Preserve uncommitted work, persistent data, license and attribution.
- Preserve others' edits. Work in the assigned worktree and branch; target `dev` for PRs. Do not switch branches, merge, publish or run production operations without authorization.
- Run the smallest meaningful checks for changed behavior. Report exact commands, outcomes and limits honestly; never fabricate evidence. Stop when acceptance is satisfied.
- Keep repository documentation in English and update authoritative documents when behavior changes. Store coordination knowledge and session findings in shared project notes, not this file or engine-local memory.
