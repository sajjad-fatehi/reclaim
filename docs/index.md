# Documentation

Reclaim has an early desktop scaffold. Product and design documents describe planned capabilities; local development guidance describes the implementation.

- [Product context](../PRODUCT.md): confirmed users, purpose and constraints.
- [Design specification](design/index.md): approved direction, information architecture, flows, layouts and proposed design tokens.
- [Development workflow](development/workflow.md): specification-driven delivery, architecture boundaries, safety and verification.
- [Local development](development/local-setup.md): requirements, installation, checks, local build and code ownership.
- [Desktop scaffold](specs/desktop-scaffold.md): scope, safety and verification evidence.
- [Specification template](specs/template.md): lightweight behavior and acceptance record.
- [Coding-agent rules](../AGENTS.md): concise repository-wide entry point.

## Documentation conventions

- Keep English repository documentation under `docs/`, except root entry points and standard repository files.
- Put development guidance in `development/`, behavior specifications in `specs/`, and interface specifications in `design/`. Add architecture or decision documents only when actual content needs them; do not create empty topic folders.
- Keep one authoritative source per topic. Update it when behavior, contracts, safety constraints or setup steps change; link to it rather than copying its rules.
- Use descriptive filenames and relative links. Link new documents here or through a topic entry point linked here; check links after moving or renaming documents.

Application setup lives in local development guidance above. Contribution guidance remains a separate board task. The onboarding task owns root `README.md` and `CONTRIBUTING.md`; link this index from those entry points when they exist.
