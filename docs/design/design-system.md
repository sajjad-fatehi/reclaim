# Proposed design system

[Design index](index.md)

Status: proposed starting values for the user's warm CleanMyMac-inspired direction. These replace the cold Space Layers tokens; the actual implementation may tune them while preserving contrast and the committed reference character. See [reference packet](warm-reference.md). Use system appearance initially; both themes have equal status.

## Color roles

| Semantic role | Light | Dark |
| --- | --- | --- |
| Canvas | `#FBF5FA` | `#211626` |
| Surface | `#FFFDFC` | `#302135` |
| Raised surface | `#F3E7F2` | `#3C2A40` |
| Navigation ground | `#38233F` | `#28192F` |
| Navigation text | `#FFF4FD` | `#FFF4FD` |
| Primary text | `#34243C` | `#FFF4FD` |
| Secondary text | `#705775` | `#D6BBD5` |
| Decorative divider | `#DDC9DF` | `#6A4B6D` |
| Interactive boundary | `#98739C` | `#B28EB5` |
| Action / focus | `#8B3A62` | `#FFBAA1` |
| On action | `#FFFFFF` | `#3F213A` |
| Selection ground | `#F3D6E7` | `#59374D` |
| Selection text | `#66304D` | `#FFE3EE` |
| Warning text | `#92400E` | `#FEC84B` |
| Danger text | `#A32E40` | `#FFB6BF` |
| Success text | `#226A4A` | `#8ED5B0` |

Warning, danger and success text above is intended on Surface. Introduce background variants only after checking the exact pair. Decorative dividers are not the sole boundary of controls. Use Interactive boundary when the edge itself carries an affordance.

Proposed data roles use lilac, peach, berry and complementary cool contrast where necessary for chart differentiation; free space remains a clearly labelled neutral field. Supply a persistent text legend, accessible names and distinguishable borders/pattern alternatives. Color never implies deletion eligibility. Unknown is not a warning severity by itself.

Use a coherent warm gradient/material language at shell and feature-surface scale, original dimensional artwork and soft controlled shadows/highlights. Translucency is optional and must preserve contrast. Keep dense file tables crisp; do not apply decorative blur or glossy texture behind every cell. Expressive color is intentional, not forbidden by generic anti-slop defaults.

## Typography and number formatting

- UI family: system sans-serif stack, using the macOS system face through CSS. No remote font request.
- Path/code fallback: system monospace; do not set the whole interface in monospace.
- Proposed sizes: 12 px metadata, 13 px table data, 14 px body/control, 16 px subsection, 22 px section, 30–34 px key contextual heading. A friendly generous overview can coexist with precise dense inspection; preserve hierarchy and avoid marketing slogans.
- Weights: 400 body, 500 controls, 600 headings. Use tabular numerals for comparable sizes and right-align numerical columns.
- Body line-height 1.5; table/control line-height at least 1.35 with room for scaling.
- Keep byte values numeric internally. Label displayed units consistently; do not silently compare GB and GiB. Expose exact bytes in item details and identify measurement basis/time.

## Geometry

- Spacing scale: 4, 8, 12, 16, 24, 32, 48 px.
- Radius scale: 8 px compact controls, 12 px buttons, 18–24 px feature surfaces and overlays. Use tighter table geometry within the warm shell; keep material/rounding systematic rather than making every row a separate card.
- Borders: 1 px by default; a visible 2 px focus outline with separation from the control.
- Icons: one vector family, consistent stroke and 16/20 px sizes. Decorative icons are hidden from assistive technology; icon-only controls have names and tooltips.
- Use 32–36 px practical desktop control heights where feasible. Ensure at least 24 × 24 CSS px pointer targets or suitable spacing; dense controls still need usable hit areas.

## States and motion

Define default, hover, pressed, focus-visible, selected, disabled, pending and invalid states for interactive components. Disabled state includes real semantics and an explanation where its reason is not obvious.

The signature interaction links a selected usage segment to its list and inspector. It preserves context and has an equivalent textual/keyboard route. Bar changes reflect actual measurements; no theatrical counting animation or invented scan percentage.

Motion is an intended part of the interface's character, not limited to hover fades. Use coordinated navigation, inspector, selection and scan feedback as specified in [Icons and motion](icons-and-motion.md). Proposed timing ranges are 100–150 ms for immediate feedback, 160–240 ms for state changes, and 220–320 ms for panels and view continuity. Keep exits faster than entrances and interactions available throughout. Reduced motion preserves clear feedback with minimal spatial movement. No scroll-jacking or distracting background loops.

## Accessibility contract

- Target WCAG 2.2 AA where applicable to the WebView interface. Normal text needs 4.5:1 contrast; relevant non-text controls and states need 3:1. Small secondary text does not get a lower requirement.
- Use real headings, labels, buttons, tables and dialogs. Choose a complex ARIA grid only if the interaction actually requires it and its keyboard model is implemented.
- Natural DOM focus order follows visual order. No positive tabindex, keyboard traps or shortcuts that replace discoverable controls.
- Proposed shortcuts: Command-F focuses current-view search; Escape closes the top dismissible overlay; table navigation follows the chosen semantic pattern. Do not hijack macOS reserved shortcuts.
- Preserve usable layout at 200% text enlargement. Test VoiceOver in the macOS WebView; a browser-only check is insufficient.
- Focus rings remain visible against each adjacent surface. Recheck actual rendered pairs, chart states, disabled text and overlays during implementation.

## Ownership and implementation boundary

When implemented, semantic tokens belong in one shared frontend theme source. Screens consume roles rather than per-screen colors. Keep domain eligibility and filesystem authority in Rust; frontend status styling cannot grant permission to delete.

No executable CSS, component package, font dependency, chart library or token generator is added by this documentation phase.
