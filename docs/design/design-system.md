# Proposed design system

[Design index](index.md)

Status: implemented in `src/shell/tokens.css` as the Atlas direction, a visualization-led desktop canvas. These values supersede the earlier warm plum tokens; [DESIGN.md](../../DESIGN.md) records the shipped tokens and components. Use system appearance initially; both themes have equal status.

## Color roles

| Semantic role             | Light                           | Dark                                             |
| ------------------------- | ------------------------------- | ------------------------------------------------ |
| Canvas                    | `#F3F8FC` plus pale sky glows   | `#0C1722` plus deep sky glows                    |
| Surface (opaque panel)    | `#FFFFFF`                       | `#14232F`                                        |
| Raised surface            | `#E6F0F8`                       | `#1C2F3E`                                        |
| Glass / strong glass      | `rgb(255 255 255 / .72)` / `.9` | `rgb(255 255 255 / .07)` / `rgb(20 35 47 / .94)` |
| Edge                      | `rgb(20 60 100 / .16)`          | `rgb(255 255 255 / .14)`                         |
| Primary text              | `#12202E`                       | `#EEF5FB`                                        |
| Secondary text            | `#475A6C`                       | `#A9BCCD`                                        |
| Control boundary          | `#6C869B`                       | `#7F97AB`                                        |
| Accent fill / accent text | `#0577B8` / `#075F94`           | `#38B6F5` / `#7CC8FB`                            |
| On accent                 | `#FFFFFF`                       | `#06202F`                                        |
| Selection ground / text   | `#D6ECFA` / `#0B5A8A`           | `#1C3A52` / `#E6F4FF`                            |
| Warning text / surface    | `#7A4A00` / `#FFF0D0`           | `#F6CD78` / `#33270E`                            |
| Danger text               | `#A8261C`                       | `#FF958B`                                        |
| Success text              | `#0C6B36`                       | `#86E3A5`                                        |

Warning, danger and success text above is intended on surface or glass over the canvas; check the exact pair before introducing another background. Decorative dividers are not the sole boundary of a control; use the control boundary when the edge carries an affordance.

### Data colors

Seven categories (dependencies, build, docker, stores, editor, worktrees, logs) plus unknown, in two independent sets so text contrast and graphic contrast are tuned separately:

| Category     | Tile light | Chart light | Tile and chart dark |
| ------------ | ---------- | ----------- | ------------------- |
| Dependencies | `#C4C9FF`  | `#4A55D6`   | `#AAB4FF`           |
| Build        | `#FFCD96`  | `#C96A00`   | `#FFBF80`           |
| Docker       | `#8FE3D0`  | `#0B8F78`   | `#6FDCC4`           |
| Stores       | `#E8CFA8`  | `#9A6420`   | `#E8C48C`           |
| Editor       | `#CFE99A`  | `#5D8F17`   | `#BDE37A`           |
| Worktrees    | `#FFB6CD`  | `#D82F60`   | `#FF9DBB`           |
| Logs         | `#D3DAE2`  | `#64758A`   | `#C3CCD6`           |
| Unknown      | `#E3E8EE`  | `#5F6F82`   | `#AAB5C1`           |

Text on tiles is `#10202E` in both themes. Chart fills keep at least 3:1 against the light canvas; dark fills are bright on a dark canvas. Every color has a persistent text label. Unknown and outside-scope areas use a hatch, never a plain fill, and color never implies deletion eligibility.

Use translucency and soft glows at shell and panel scale. Keep dense tables crisp: no blur or gradient behind data cells.

## Typography and number formatting

- Display family: the system rounded face through `ui-rounded`, falling back to system sans, for titles, figures and the brand. UI family: system sans. Path family: system monospace. No web font is loaded; the content security policy permits none, and adding one would need an explicit policy change.
- Sizes: 11 px count badges, 12 px metadata and status, 14 px body and controls, 17 px subtitles, 30 px screen titles, 34 px figures.
- Weights: 400 body, 500 controls and labels, 700 display text. Use tabular numerals and right alignment for comparable sizes.
- Body line-height 1.5; table and control line-height at least 1.35 with room for scaling.
- Keep byte values numeric internally. Label displayed units consistently; do not silently compare GB and GiB. Expose exact bytes in item details and identify the measurement basis and time.

## Geometry

- Spacing scale: 4, 8, 16, 24, 32 px, with 12 px for tight control padding.
- Radius scale: pill for buttons, tabs and chips; 12 px fields and choices; 14 px tiles; 20 px panels; 26 px dialogs.
- Borders: 1 px hairline edges; a visible 3 px focus outline with 2 px offset.
- Icons: one vector family, 16 to 20 px, consistent stroke. Decorative icons are hidden from assistive technology; icon-only controls have names and tooltips.
- Controls are at least 40 px tall; the 36 px navigation tab sits inside a padded pill, which keeps every target at least 24 × 24 CSS px with spacing.

## States and motion

Define default, hover, pressed, focus-visible, selected, disabled, pending and invalid states for interactive components. Disabled state includes real semantics and an explanation where its reason is not obvious.

The signature interaction links a selected chart slice or tile to its list and inspector. It preserves context and has an equivalent textual/keyboard route. Bar changes reflect actual measurements; no theatrical counting animation or invented scan percentage.

Motion is an intended part of the interface's character, not limited to hover fades. Use coordinated navigation, inspector, selection and scan feedback as specified in [Icons and motion](icons-and-motion.md). Shipped durations are 120 ms for immediate feedback, 200 ms for state changes, 320 ms for panels and 800 ms for chart sweeps, with an out-curve and a spring curve reserved for navigation and buttons (see [Icons and motion](icons-and-motion.md)). Keep exits faster than entrances and interactions available throughout. Reduced motion preserves clear feedback with minimal spatial movement. No scroll-jacking or distracting background loops.

## Accessibility contract

- Target WCAG 2.2 AA where applicable to the WebView interface. Normal text needs 4.5:1 contrast; relevant non-text controls and states need 3:1. Small secondary text does not get a lower requirement.
- Use real headings, labels, buttons, tables and dialogs. Choose a complex ARIA grid only if the interaction actually requires it and its keyboard model is implemented.
- Natural DOM focus order follows visual order. No positive tabindex, keyboard traps or shortcuts that replace discoverable controls.
- Proposed shortcuts: Command-F focuses current-view search; Escape closes the top dismissible overlay; table navigation follows the chosen semantic pattern. Do not hijack macOS reserved shortcuts.
- Preserve usable layout at 200% text enlargement. Test VoiceOver in the macOS WebView; a browser-only check is insufficient.
- Focus rings remain visible against each adjacent surface. Recheck actual rendered pairs, chart states, disabled text and overlays during implementation.

## Ownership and implementation boundary

Semantic tokens belong in one shared frontend theme source, `src/shell/tokens.css`. Screens consume roles rather than per-screen colors. Keep domain eligibility and filesystem authority in Rust; frontend status styling cannot grant permission to delete.

The foundation adds no font, chart, animation or token-generator dependency.
