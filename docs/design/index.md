# Reclaim design specification

## Status and authority

These specifications govern implementation. The working desktop shell's actual tokens and components are recorded in [the built design system](../../DESIGN.md); feature states remain planned until their owning specifications have passing evidence.

Confirmed by the user:

- React + TypeScript inside Tauri 2, with a Rust core; macOS first.
- Overview, Explore, Development, and Review & Clean as the main sections.
- English product copy initially.
- Suggested scan locations are reviewed and edited before the user starts scanning.
- **Atlas** as the current visual direction: a visualization-led canvas with floating pill navigation, glass panels, pale and deep sky-blue grounds and expressive, bounded motion. It supersedes the warm plum direction and Space Layers; the shipped tokens are recorded in [DESIGN.md](../../DESIGN.md).
- Expressive, coherent icons and authored motion are desired parts of the interface, not optional finishing decoration.

The detailed layouts, tokens, copy and keyboard choices below are a proposed implementation specification. They still require rendered validation; they are not a claim of user approval for every detail or of tested accessibility.

## Design thesis

Make unexplained storage inspectable. Let people move from a volume's usage to a path, its owner, the evidence and the consequences of removal without losing their place.

Use the Atlas world: glass panels on a pale sky-white or deep navy canvas, a color per kind of storage shared by every chart and label, and coordinated bounded motion. A selected chart slice or tile links to its list row and inspector because it explains real data. The earlier [warm reference](warm-reference.md) is superseded and kept only as history.

Assumed usage scene: someone investigating storage pressure on a laptop in ordinary daytime or evening light. Follow system light/dark appearance and keep both equally legible. Expression applies to both themes; the app should feel welcoming rather than like a cold administrative console.

## Specification map

- [Information architecture](information-architecture.md)
- [Interaction flows and states](interaction-flows.md)
- [Layouts and component inventory](layouts.md)
- [Proposed design system](design-system.md)
- [Icons and motion](icons-and-motion.md)
- [Superseded warm visual direction (history)](warm-reference.md)
- [Built shell design system](../../DESIGN.md)

## Method and anti-slop rules

Use ui-ux-pro-max as a source of applicable usability guidance and impeccable's Operate mode for product-specific clarity and craft. Generated recommendations are evidence to evaluate, not decisions to copy. The initial generator's hero/features/CTA layout was rejected as a landing-page pattern.

- Real work leads: paths, ownership, size, coverage and consequences.
- Rich gradients, rounded task surfaces, subtle translucency and soft highlight/depth are allowed when they serve the user-pinned reference. Avoid repeated empty KPI cards and decorative treatments that obscure data or controls.
- Use a table for comparisons and a chart only when it explains proportions. Never let a chart be the only route to data.
- Do not turn the app into a terminal, scientific instrument or geological illustration.
- Do not score device health, celebrate arbitrary cleanup totals or use fear to encourage deletion.
- Keep one clear primary action per local task. Cleanup requires review, not a persistent pressure-inducing button.

## Superseded direction evaluation record

The following is historical, not current visual authority. The user subsequently replaced this direction with the warm CleanMyMac-inspired brief in warm-reference.md.

Seven grounded references considered: technical documentation; transit maps; library classification; component packaging; stratigraphic charts; contour maps; photographic contact sheets. The selected fifth reference became Space Layers. The alternative technical direction was presented with its familiarity risk, alongside the conventional macOS option. The user selected Space Layers.

Catalog challengers were assessed for audience identification and product clarity. Split-flap boards, airport signs, vertical video feeds, star atlases, bitmap specimens and ticket wallets were declined: none improved both axes over a standard product shell with readable storage relationships. Retained disciplines, without their visual costumes:

- Split-flap board: stable columns during updates.
- Airport signs: explicit next-step labels.
- Vertical feed: a clear selected-item focus without discarding comparison.
- Star atlas: proportions encode quantities consistently.
- Bitmap specimen: scale hierarchy stays systematic, without bitmap text.
- Ticket wallet: completed operations remain inspectable in history.

Direction reference: `seed:7aa891e52a1d6f6a`. This records the design session, not an implementation dependency.

## Delivery and open decisions

The working shell's rendered system is recorded in [DESIGN.md](../../DESIGN.md), with runtime tokens in `src/shell/tokens.css` and native evidence in [the shell specification](../specs/features/application-shell.md). The [read-only native scan producer](../specs/features/scan-foundation.md) is implemented independently; its production scan views and later cleanup remain pending. Proposed feature layouts are not claims of implemented scan UI.

Still open: recognizer priorities; measurement feasibility; actual memory/performance budgets; supported macOS versions; component library choice; final minimum window dimensions; usability evidence. Do not invent a technical guarantee to complete a mockup.

Implementation acceptance: verify all named states, actual macOS WebView behavior, keyboard and VoiceOver access, both themes, reduced motion, long paths, large result sets and smaller desktop windows. Inspect in one batched round, fix material findings together and confirm once. Documentation acceptance alone cannot satisfy that future gate.
