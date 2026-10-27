---
name: Reclaim
description: Atlas — a visualization-led desktop canvas where space is a shape you can read.
colors:
  canvas: '#fbf5ec'
  surface: '#fffcf7'
  raised: '#f4ece2'
  text: '#241a33'
  secondary: '#5d5270'
  boundary: '#8c7fa3'
  accent: '#5b3fd6'
  accent-text: '#4a2fc0'
  on-accent: '#ffffff'
  selection: '#e6defb'
  selection-text: '#3a2394'
  warning: '#7a4a00'
  warning-surface: '#fff0d0'
  danger: '#a8261c'
  success: '#0c6b36'
  tile-text: '#1c1626'
  canvas-dark: '#120e1c'
  surface-dark: '#221b34'
  raised-dark: '#2c2342'
  text-dark: '#f4f0fb'
  secondary-dark: '#bdb4d1'
  boundary-dark: '#8f84ab'
  accent-dark: '#7a5cf0'
  accent-text-dark: '#c0b0ff'
  selection-dark: '#3a2d66'
  selection-text-dark: '#e6dcff'
  warning-dark: '#f6cd78'
  warning-surface-dark: '#33270e'
  danger-dark: '#ff958b'
  success-dark: '#86e3a5'
typography:
  display:
    fontFamily: "ui-rounded, 'SF Pro Rounded', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif"
    fontSize: '1.875rem'
    fontWeight: 700
    lineHeight: 1.1
    letterSpacing: '-0.01em'
  subtitle:
    fontFamily: "ui-rounded, 'SF Pro Rounded', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif"
    fontSize: '1.0625rem'
    fontWeight: 700
    lineHeight: 1.5
  body:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif"
    fontSize: '0.875rem'
    fontWeight: 400
    lineHeight: 1.5
  label:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif"
    fontSize: '0.75rem'
    fontWeight: 500
    lineHeight: 1.5
rounded:
  control: '12px'
  tile: '14px'
  panel: '20px'
  dialog: '26px'
  pill: '999px'
spacing:
  hairline: '4px'
  small: '8px'
  medium: '16px'
  large: '24px'
  section: '32px'
components:
  button-default:
    backgroundColor: '{colors.surface}'
    textColor: '{colors.text}'
    rounded: '{rounded.pill}'
    padding: '0.4rem 1rem'
  button-primary:
    backgroundColor: '{colors.accent}'
    textColor: '{colors.on-accent}'
    rounded: '{rounded.pill}'
    padding: '0.4rem 1rem'
  navigation-pill:
    rounded: '{rounded.pill}'
    padding: '5px'
  navigation-tab-current:
    backgroundColor: '{colors.accent}'
    textColor: '{colors.on-accent}'
    rounded: '{rounded.pill}'
    padding: '0.4rem 0.9rem'
  status-chip:
    textColor: '{colors.text}'
    rounded: '{rounded.pill}'
    padding: '5px 12px'
  search-field:
    backgroundColor: '{colors.surface}'
    textColor: '{colors.text}'
    rounded: '{rounded.control}'
    padding: '0.4rem 0.75rem'
    width: 'min(100%, 18rem)'
  glass-panel:
    rounded: '{rounded.panel}'
    padding: '{spacing.large}'
  dialog:
    rounded: '{rounded.dialog}'
    padding: '24px 28px'
---

# Design System: Reclaim

## Overview

**Creative North Star: "Atlas"**

The window is a canvas, not a chrome frame. A floating pill carries navigation, glass panels sit on a warm or violet-lit ground, and measured space is drawn as shapes: category colors are shared between charts, tiles, legends, chips and dialogs so a color always means the same kind of storage. Motion is expressive but bounded and never carries meaning alone.

Runtime values live in `src/shell/tokens.css`; this document records them and does not define a second theme. Screens consume semantic roles and never introduce per-screen colors. Honesty rules for charts (outside-scope and unrecognized areas hatched, never drawn as if measured) are part of the system, see [design system](docs/design/design-system.md).

**Key Characteristics:**

- Floating pill navigation and a header with scan-coverage status; no sidebar.
- Warm cream (light) and deep violet (dark) canvases with soft color glows.
- Glass panels with a hairline edge and backdrop blur; dense tables stay crisp.
- Separate saturated chart fills and pastel tile fills, tuned independently.
- Pill controls, large radii, spring-eased arrival of the current tab.

## Colors

Frontmatter names follow the runtime roles. `-dark` entries are theme overrides; roles not listed inherit the light value.

### Roles

- **Accent:** primary action, current tab, focus emphasis, measured scope. `accent-text` is the variant for text and outlines on the canvas; `on-accent` is the foreground on accent fills.
- **Selection:** selected rows and appearance choices, always paired with `selection-text`.
- **Canvas / surface / raised:** ground, opaque panel and control hover. The canvas carries two radial glows (`--canvas-glow`): peach and lilac in light, violet and plum in dark.
- **Glass / glass-strong / edge:** translucent panel, near-opaque overlay and the hairline around both. Glass is for framing; it never sits behind dense cells.
- **Text / secondary:** reading hierarchy. **Boundary** is the 3:1 outline for controls that must be discoverable; **divider** is decorative.
- **Warning / danger / success:** text roles checked on surface. `warning-surface` and `danger-surface` are the fills for caution chips and destructive buttons.

### Data color sets

Seven category hues (dependencies, build, docker, stores, editor, worktrees, logs) plus an unknown ground, each in two sets:

- **Tile fills** (`--tile-*`) are pastel in light and bright in dark. Text on them is always `--tile-text` (`#1c1626`) and reaches 4.5:1.
- **Chart fills** (`--chart-*`) are saturated in light so graphic marks keep at least 3:1 against the canvas; in dark they equal the tile fills.

Unknown (`--tile-unknown`, `--chart-unknown`) is rendered hatched, never as a plain slice. Every color sits beside a text label.

### Named Rules

**The Paired Theme Rule.** Change semantic colors together through the runtime theme; both appearances are first class.

**The Truthful Color Rule.** Color never implies safety to delete; unknown or unmeasured space keeps a hatch or dashed treatment.

## Typography

**Display:** the macOS rounded system face (`ui-rounded`, falling back to system sans) for titles, numbers and the brand. **Body:** system sans. **Mono:** system monospace, for paths only. No web font is loaded: the content security policy forbids remote and local font files, and the stacks give a close rounded character natively.

### Hierarchy

- **Title:** 1.875rem, 700, tight tracking, balanced wrapping (screen titles).
- **Figure:** 2.125rem display numerals for inspector and chart centers.
- **Subtitle:** 1.0625rem, 700, panel and table titles.
- **Body:** 0.875rem, line-height 1.5; prose capped at 65ch.
- **Label:** 0.75rem for status chips and footer; micro 0.6875rem for count badges.
- Numbers use tabular numerals and right alignment in comparable columns.

## Layout

The shell is a flex column: a three-part window header (brand, pill navigation, status), a flexible workspace and a status footer. The header is a `1fr auto 1fr` grid so the pill stays centered; at 1100px and below the pill moves to its own row. At 760px and below inactive tab labels collapse to icons while the current tab keeps its label. The visible screen title is owned by each screen; the shell supplies a visually hidden `h1` with the destination name.

Views scroll independently and keep their scroll position when you leave and return. Standard view inset is `24px`, becoming `16px` at 1100px. Inspector is an inline 22rem glass panel above 1100px and an edge sheet at or below it. Results keep a 32rem table minimum and scroll inside a labelled focusable region. Minimum body width is 360px.

## Elevation & Depth

Depth comes from translucency, hairline edges and soft shadows.

- **Control:** small lift for the navigation pill and primary actions.
- **Panel:** diffuse lift for inspectors and large glass panels.
- **Overlay:** deep shadow for dialogs over a blurred scrim.

### Named Rules

**The Quiet Depth Rule.** Rows stay flat; only containers, tiles and overlays lift.

## Shapes

Pill radius for buttons, tabs, chips and status; 12px for fields and choices; 14px for tiles; 20px for panels; 26px for dialogs. Edge sheets round their inner corners only. Buttons and fields keep a 40px minimum height; the navigation tab is 36px inside a pill that provides spacing.

## Components

### Buttons

Pill on `glass-strong` with an edge hairline. Hover picks up selection tint and rises 1px; press scales to 0.96; disabled drops to 55% opacity and shows a not-allowed cursor. Primary uses accent/on-accent with a control shadow. Hover and press movement ends under reduced motion. Focus is a 3px `accent-text` outline with 2px offset.

### Navigation

Floating glass pill with four tabs: Overview, Explore, Development, Review & Clean. The current tab fills with accent, uses a filled icon and squishes once on arrival. Review & Clean carries a count badge that is also part of the accessible name. Settings is a round icon button to the right of the pill. A skip link precedes the header.

### Window status

A chip reads the scan state in words: not started, scanning (pulsing icon), complete, partial coverage, cancelled, failed or stale. Anything other than a fresh complete scan uses the caution surface and a warning icon, so partial coverage is visible from every screen.

### Panels, notices and tables

Glass panels share one rule (`glass`, edge, panel radius). Notices are compact glass rows. Tables are collapsed with divider rows, muted headings and selection fill. Dialogs use `glass-strong`, dialog radius, overlay shadow and a blurred scrim; Close receives focus, Escape closes and focus returns to the invoker.

### Fields and choices

Search and select fields use surface, a boundary outline and control radius. Native radios stay real inputs; the selected choice uses selection colors with an accent border.

### Original folder artwork

Use [the original alpha WebP](assets/illustrations/reclaim-folder.webp), with [prompt provenance](assets/illustrations/reclaim-folder.prompt.txt), only as optional decorative art beside equivalent text.

## Motion

| Token | Value | Use |
| --- | --- | --- |
| `--duration-feedback` | 120ms | hover and press color |
| `--duration-selection` | 200ms | selection, view entrance, spring press |
| `--duration-panel` | 320ms | inspector, dialog |
| `--duration-chart` | 800ms | chart sweep (screens) |
| `--duration-artwork` | 300ms | illustration settle |
| `--ease-out` | `cubic-bezier(0.2, 0.9, 0.25, 1)` | default |
| `--ease-spring` | `cubic-bezier(0.3, 1.6, 0.5, 1)` | tab arrival, button press |

View entrance moves 6px from opacity 0.6; edge panels move 20px. The spring easing is reserved for navigation and buttons, never for data. `prefers-reduced-motion: reduce` removes every transition and animation and the hover/press transform; end states render immediately and state, focus and text remain.

## Do's and Don'ts

### Do:

- **Do** reuse semantic tokens and test both appearances.
- **Do** keep text labels, visible focus and native form semantics.
- **Do** keep chart and tile fills in their own token sets.
- **Do** label unmeasured and unknown space and keep it hatched or dashed.

### Don't:

- **Don't** ship competitor screenshots, symbols or branding.
- **Don't** fabricate scan metrics or celebrate deletion with motion.
- **Don't** put glass or blur behind dense table cells.
- **Don't** use color alone to state a category, risk or eligibility.
