# Reclaim design specification

## Status and authority

Documentation only. No prototype or application UI is implemented by this work.

Confirmed by the user:

- React + TypeScript inside Tauri 2, with a Rust core; macOS first.
- Overview, Explore, Development, and Review & Clean as the main sections.
- English product copy initially.
- Suggested scan locations are reviewed and edited before the user starts scanning.
- **Warm, modern, playful/fantastical desktop design inspired by CleanMyMac's craft** as the latest visual direction. This supersedes Space Layers.
- Expressive, coherent icons and authored motion are desired parts of the interface, not optional finishing decoration.

The detailed layouts, tokens, copy and keyboard choices below are a proposed implementation specification. They still require rendered validation; they are not a claim of user approval for every detail or of tested accessibility.

## Design thesis

Make unexplained storage inspectable. Let people move from a volume's usage to a path, its owner, the evidence and the consequences of removal without losing their place.

Use a warm plum/lilac/peach world with sculptural original artwork, friendly color, tactile depth and coordinated motion. The user's pinned CleanMyMac reference wins over the earlier geological slate/teal/ochre proposal. A segmented usage strip still links selection to the corresponding list and inspector because it explains real data, not because the old aesthetic remains binding. Follow [the verified visual reference packet](warm-reference.md).

Assumed usage scene: someone investigating storage pressure on a laptop in ordinary daytime or evening light. Follow system light/dark appearance and keep both equally legible. Warmth and expression apply to both themes; the app should feel welcoming rather than like a cold administrative console.

## Specification map

- [Information architecture](information-architecture.md)
- [Interaction flows and states](interaction-flows.md)
- [Layouts and component inventory](layouts.md)
- [Proposed design system](design-system.md)
- [Icons and motion](icons-and-motion.md)
- [Warm visual direction and verified references](warm-reference.md)

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

No DESIGN.md or executable token package is generated now. A future implementation records the system actually built in DESIGN.md after rendered review, rather than treating this proposed palette as proven.

Still open: recognizer priorities; measurement feasibility; actual memory/performance budgets; supported macOS versions; component library choice; final minimum window dimensions; usability evidence. Do not invent a technical guarantee to complete a mockup.

Implementation acceptance: verify all named states, actual macOS WebView behavior, keyboard and VoiceOver access, both themes, reduced motion, long paths, large result sets and smaller desktop windows. Inspect in one batched round, fix material findings together and confirm once. Documentation acceptance alone cannot satisfy that future gate.
