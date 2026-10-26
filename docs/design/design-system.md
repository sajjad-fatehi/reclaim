# Proposed design system

[Design index](index.md)

Status: proposed semantic values for Space Layers. Not an implemented or fully accessibility-tested system. Use system appearance initially; both themes have equal status.

## Color roles

| Semantic role | Light | Dark |
| --- | --- | --- |
| Canvas | `#F3F6F8` | `#111A22` |
| Surface | `#FFFFFF` | `#182530` |
| Raised surface | `#EAF0F4` | `#20303D` |
| Navigation ground | `#182F46` | `#142A3D` |
| Navigation text | `#F4F8FC` | `#F4F8FC` |
| Primary text | `#172B3A` | `#EDF3F8` |
| Secondary text | `#465D70` | `#B6C5D2` |
| Decorative divider | `#CED8E0` | `#394B5B` |
| Interactive boundary | `#718497` | `#8197AA` |
| Action / focus | `#0F766E` | `#66D4C5` |
| On action | `#FFFFFF` | `#0E2523` |
| Selection ground | `#D9F0EB` | `#21443F` |
| Selection text | `#144C46` | `#B5F5E6` |
| Warning text | `#92400E` | `#FEC84B` |
| Danger text | `#B42318` | `#FDA29B` |
| Success text | `#18794E` | `#75E0A7` |

Warning, danger and success text above is intended on Surface. Introduce background variants only after checking the exact pair. Decorative dividers are not the sole boundary of controls. Use Interactive boundary when the edge itself carries an affordance.

Proposed data series: projects teal, shared tools blue, personal files violet, unexplained ochre, free space a neutral field. Supply a persistent text legend, accessible names and distinguishable borders/pattern alternatives. Color never implies deletion eligibility. Unknown is not a warning severity by itself.

No gradients, glow or translucent glass are required. Depth comes from surface hierarchy, spacing and optional restrained overlays. Do not scatter colored badges simply to make neutral content look designed.

## Typography and number formatting

- UI family: system sans-serif stack, using the macOS system face through CSS. No remote font request.
- Path/code fallback: system monospace; do not set the whole interface in monospace.
- Proposed sizes: 12 px metadata, 13 px table data, 14 px body/control, 16 px subsection, 20 px section, 28 px primary volume figure. Avoid oversized marketing headings.
- Weights: 400 body, 500 controls, 600 headings. Use tabular numerals for comparable sizes and right-align numerical columns.
- Body line-height 1.5; table/control line-height at least 1.35 with room for scaling.
- Keep byte values numeric internally. Label displayed units consistently; do not silently compare GB and GiB. Expose exact bytes in item details and identify measurement basis/time.

## Geometry

- Spacing scale: 4, 8, 12, 16, 24, 32, 48 px.
- Radius scale: 4 px controls, 8 px overlays/panels. Avoid rounding every table section into a floating card.
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
