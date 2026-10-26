# Specification template

Copy the sections relevant to the change into a descriptive specification file. Update an existing authoritative specification instead when possible. Remove inapplicable prompts; a trivial task description may already be sufficient. Link the resulting path or a precise shared project note ID from the task.

## Purpose and observable behavior

What user problem changes? Describe current and intended behavior without claiming planned work already exists.

## Scope and non-goals

What is included? What is explicitly excluded? Link related authoritative specifications instead of duplicating them.

## Acceptance criteria

- State observable, verifiable outcomes.
- Include relevant edge cases and error outcomes.

## Safety constraints

Record applicable authorization, privacy, accessibility and data-loss requirements. For destructive behavior, define protected data, exact operation scope, partial failure handling and supported recovery. Verification must use isolated fixtures, never destructive cleanup against real user data.

## Unresolved decisions

Record genuine open questions and which block implementation. Resolve blocking product/security/data choices; leave ordinary technical decisions to the implementer. State “None” when resolved.

## Verification evidence

Map acceptance criteria to focused checks. After implementation, record exact commands, results and material limitations, distinguishing planned checks from checks actually run. Update this specification when accepted behavior changes.
