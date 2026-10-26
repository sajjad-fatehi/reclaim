# Documentation

Reclaim has an early desktop scaffold and a read-only native scan producer. Product and design documents describe planned capabilities; local development guidance describes the implementation.

- [Product context](../PRODUCT.md): confirmed users, purpose and constraints.
- [Product direction](product.md): planned behavior, delivery stages and open decisions.
- [Design specification](design/index.md): approved direction, information architecture, flows, layouts and proposed design tokens.
- [Development workflow](development/workflow.md): specification-driven delivery, architecture boundaries, safety and verification.
- [Local development](development/local-setup.md): requirements, installation, checks, local build and code ownership.
- [Desktop scaffold](specs/desktop-scaffold.md): scope, safety and verification evidence.
- [Specification template](specs/template.md): lightweight behavior and acceptance record.
- [Application feature specifications](specs/features/index.md): complete approved feature scope, dependency contracts and delivery acceptance.
- [Coding-agent rules](../AGENTS.md): concise repository-wide entry point.
- [Contributing](../CONTRIBUTING.md): focused contributions, validation and branch flow.
- [Security](../SECURITY.md): current private reporting limitations.
- [Code of conduct](../CODE_OF_CONDUCT.md): participation standards and reporting limits.

## Documentation conventions

- Keep English repository documentation under `docs/`, except root entry points and standard repository files.
- Put development guidance in `development/`, behavior specifications in `specs/`, and interface specifications in `design/`. Add architecture or decision documents only when actual content needs them; do not create empty topic folders.
- Keep one authoritative source per topic. Update it when behavior, contracts, safety constraints or setup steps change; link to it rather than copying its rules.
- Use descriptive filenames and relative links. Link new documents here or through a topic entry point linked here; check links after moving or renaming documents.

Application setup lives in local development guidance above. The [README](../README.md)
provides the repository introduction and quick start.
