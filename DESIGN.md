---
name: Reclaim
description: Warm desktop surfaces for readable storage investigation.
colors:
  canvas: '#fbf5fa'
  surface: '#fffdfc'
  raised: '#f3e7f2'
  navigation: '#38233f'
  navigation-text: '#fff4fd'
  navigation-selection: '#69466b'
  text: '#34243c'
  secondary: '#705775'
  divider: '#ddc9df'
  boundary: '#98739c'
  accent: '#8b3a62'
  on-accent: '#fffdfc'
  selection: '#f3d6e7'
  selection-text: '#66304d'
  scrim: '#20122788'
  navigation-deep: '#27162f'
  navigation-icon: '#ffffff14'
  navigation-accent: '#ffccb8'
  canvas-dark: '#211626'
  surface-dark: '#302135'
  raised-dark: '#3c2a40'
  navigation-dark: '#28192f'
  navigation-selection-dark: '#69466b'
  text-dark: '#fff4fd'
  secondary-dark: '#d6bbd5'
  divider-dark: '#6a4b6d'
  boundary-dark: '#b28eb5'
  accent-dark: '#ffbaa1'
  on-accent-dark: '#3f213a'
  selection-dark: '#59374d'
  selection-text-dark: '#ffe3ee'
typography:
  headline:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif"
    fontSize: '2rem'
    fontWeight: 600
    lineHeight: 1.2
    letterSpacing: '-0.025em'
  workspace-title:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif"
    fontSize: '1.375rem'
    fontWeight: 600
    lineHeight: 1.5
  settings-title:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif"
    fontSize: '1.25rem'
    fontWeight: 600
    lineHeight: 1.5
  title:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif"
    fontSize: '1rem'
    fontWeight: 600
    lineHeight: 1.5
  body:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif"
    fontSize: '0.875rem'
    fontWeight: 400
    lineHeight: 1.5
  label:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif"
    fontSize: '0.75rem'
    fontWeight: 400
    lineHeight: 1.5
rounded:
  control: '8px'
  intermediate: '12px'
  panel: '20px'
spacing:
  small: '8px'
  medium: '16px'
  large: '24px'
  section: '32px'
components:
  button-default:
    backgroundColor: '{colors.surface}'
    textColor: '{colors.text}'
    rounded: '{rounded.control}'
    padding: '0.5rem 0.75rem'
  button-primary:
    backgroundColor: '{colors.accent}'
    textColor: '{colors.on-accent}'
    rounded: '{rounded.intermediate}'
    padding: '0.5rem 0.75rem'
  search-field:
    backgroundColor: '{colors.surface}'
    textColor: '{colors.text}'
    rounded: '{rounded.control}'
    padding: '0.5rem'
    width: 'min(100%, 18rem)'
  navigation-item:
    textColor: '{colors.navigation-text}'
    rounded: '{rounded.intermediate}'
    padding: '10px 12px'
  notice:
    backgroundColor: '{colors.surface}'
    textColor: '{colors.secondary}'
    rounded: '{rounded.intermediate}'
    padding: '{spacing.medium}'
  settings-card:
    backgroundColor: '{colors.surface}'
    rounded: '{rounded.panel}'
    padding: '{spacing.section}'
  appearance-choice:
    backgroundColor: '{colors.surface}'
    rounded: '{rounded.intermediate}'
    padding: '1rem'
  appearance-choice-selected:
    backgroundColor: '{colors.selection}'
    textColor: '{colors.selection-text}'
    rounded: '{rounded.intermediate}'
    padding: '1rem'
---

# Design System: Reclaim

## Overview

**Creative North Star: "Warm desktop investigation"**

Warm plum navigation, lilac and peach atmosphere, creamy surfaces and a local sculptural folder make storage investigation approachable. Soft task framing surrounds readable controls and comparable rows; decoration yields to content when space is scarce.

This is a post-build record of the direct user-pinned warm desktop direction, superseding Space Layers. Runtime values belong to `src/shell/tokens.css`; this document records them, not a second runtime theme. The confirmed visual authority is [the warm reference packet](docs/design/warm-reference.md), with surface composition in [the shell contract](.impeccable/surfaces/application-shell.md).

**Key Characteristics:**

- Plum rail with persistent text labels.
- Warm light and dark surfaces with restrained soft depth.
- One original decorative object; crisp inspection rows.
- Content-first compact and enlarged-text reflow.

## Colors

The palette pairs a deep plum rail with warm lilac neutrals and peach light. Frontmatter names follow runtime roles; `-dark` entries are theme overrides, not additional independent accents. Unchanged roles inherit their light values.

### Primary

- **Berry / peach accent:** primary action, input caret, native control accent and focus outline. Pair with on-accent text rather than assuming white text in dark mode.
- **Rose selection:** selected rows and appearance choices; use selection-text as its foreground.

### Neutral

- **Cream / plum canvas:** application ground; surface lifts notices, fields, settings and panels; raised supplies control hover.
- **Deep plum navigation:** rail gradient ends at navigation-deep; navigation-text stays bright in both modes. Navigation-selection marks the current destination; navigation-accent colors icon holders.
- **Plum ink and muted mauve:** text and secondary establish readable hierarchy. Divider separates rows; boundary outlines fields and controls. Scrim dims content behind modal panels.

### Named Rules

**The Warm Frame Rule.** Use expressive gradients and depth for task framing; preserve crisp comparable rows for inspection.

**The Paired Theme Rule.** Change semantic colors together through the runtime theme; dark appearance remains warm plum.

## Typography

**Body and UI font:** platform system sans with Segoe UI and generic sans fallbacks. Phosphor vector icons carry the small interface symbols; text labels convey destination names.

**Character:** familiar desktop reading with sentence-case labels and semibold headings. No separate editorial display family or mono family is implemented.

### Hierarchy

- **Headline:** contextual empty-state heading; recorded at 2rem, weight 600, line-height 1.2 and tight tracking. At viewport widths up to 1000px it becomes 1.75rem; measure is at most 16ch.
- **Workspace title:** 1.375rem, weight 600; brand uses 1.125rem. Settings title uses 1.25rem; ordinary section titles use 1rem.
- **Body:** 0.875rem in the shell, line-height 1.5; explanatory text is limited to 65ch where applicable.
- **Label:** 0.75rem for scan context, rail note and connection status. Navigation stays at body size; current destination becomes semibold.
- **Numbers:** right aligned with tabular numerals in comparable result columns.

The contextual headline uses the approved platform system face, consistent with the operational macOS interface and the rest of its typography.

## Layout

Desktop shell fills the dynamic viewport with a labelled rail and flexible workspace. Rail width is `max(168px, min(13rem, 30vw))`; standard captures show 208px. Header and brand have an 88px minimum height. Settings anchors to the rail bottom when room permits; the rail itself scrolls when needed.

The workspace keeps header and connection status outside its independently scrolling active view. Navigation preserves each mounted view's scroll position. Main spacing follows the small/medium/large/section scale. Standard horizontal workspace inset is section; it becomes large at 1100px and medium at 600px. Toolbar and appearance choices wrap.

The contextual overview has a 368px minimum height. A content-container width of at least 38rem enables its text/art columns (`minmax(0, 1fr) minmax(9rem, 38%)`) and reveals the illustration. Below that threshold art is absent; text, notices and actions remain scrollable, including at 200% root text. The art caps at 24rem and preserves its square aspect ratio. Body minimum width is 360px; no separate mobile navigation is implemented.

Inspector is an inline 22rem panel above 1100px; at or below that width it becomes an edge modal sheet capped by viewport width. Results retain a 32rem table minimum and scroll horizontally within a labelled focusable region. Long filenames wrap; numeric columns and row actions remain comparable.

## Elevation & Depth

A hybrid of tonal layers and soft shadows creates warmth. The rail and contextual task surface use gradients; fields use visible boundaries; notices remain quiet. These gradients are native to this chosen world.

### Shadow Vocabulary

- **Panel:** runtime shadow-panel; diffuse lift for contextual surfaces, settings, inspector and dialog (light: `0 16px 48px #4a233312`; dark: `0 16px 48px #130b1c33`).
- **Control:** runtime shadow-control; compact lift for primary actions, context, icon holders and navigation indicator (light: `0 3px 8px #3d203022`; dark: `0 3px 8px #130b1c44`).

### Named Rules

**The Quiet Depth Rule.** Use soft panel and control shadows to group and lift surfaces; keep result rows flat.

## Shapes

Controls and search fields use control radius; notices, navigation, primary actions and appearance choices use the intermediate radius; task surfaces, settings and panels use panel radius. Edge inspector sheets round their left corners only. Table rows share straight geometry, a divider and selection fill instead of individual rounded containers. Buttons and search fields retain a 40px minimum height; native checkboxes are 1.5rem square.

## Components

### Buttons

Tactile, readable controls. Default button: surface background, ink foreground, visible boundary and control radius; primary: accent/on-accent with intermediate radius and control shadow. Both use 0.5rem 0.75rem padding and at least 40px dimensions. Hover raises default tone; primary hover brightens its existing accent. Press scales to 0.96; disabled opacity is 0.65 with no press transform. Focus uses an accent outline (2px, 3px offset).

### Cards / Containers

Warm framing for tasks, not individual data items. Settings uses surface, panel radius, panel shadow and section padding (large at 760px and below). Introduction uses the feature gradient and section padding, reduced to large at 1000px and below. Notices use surface, secondary text, intermediate radius and medium padding; an informational vector icon is decorative beside text.

### Inputs / Fields

Search uses surface/text, a boundary stroke, control radius, 0.5rem padding and width capped at 18rem. It shares visible focus with buttons. Native radio choices keep their real inputs: selected wrappers use selection/selection-text with accent border; hover uses raised. Choices wrap instead of shrinking labels to illegibility. No custom error-field appearance is implemented.

### Navigation

Persistent plum rail with coherent Phosphor icons and readable labels. Rows use a minimum of `max(52px, calc(1.5em + 20px))`, intermediate radius and 10px 12px padding. Current destination uses a moving selection backplate, semibold text and filled icon. Hover uses navigation-selection; keyboard focus uses navigation-text outline inset 3px. Navigation press has no scale. Compact layouts retain labels and scroll access.

### Results and panels

Results use a full-width collapsed table, 10px 12px cells, muted medium-weight headings, divider lines and semantic selection fill. Toolbar is a wrapping row with small gaps. Dialogs and inspectors use warm surface, panel radius, large padding and panel shadow. Centered dialogs cap at 34rem and viewport minus 2rem; modal scrim and scroll containment separate them from the background. Close receives focus; Escape closes; focus returns to the invoker.

### Original folder artwork and motion

Use [the original alpha WebP](assets/illustrations/reclaim-folder.webp), with [exact prompt provenance](assets/illustrations/reclaim-folder.prompt.txt), as optional foreground art. It is hidden from assistive technology beside equivalent text; it never represents functional UI or measured results.

Feedback lasts 120ms; selection and view entrance 200ms; panels 260ms; artwork 300ms. Selection, panels and artwork use the authored ease-out curve. View entrance moves 6px vertically from opacity 0.6; panels move 20px horizontally. Artwork settles from a small downward offset and rotation. Animation cancels when interrupted; reduced motion removes animation, transitions and press scale while preserving state and focus.

## Do's and Don'ts

### Do:

- **Do** reuse semantic runtime tokens and test both appearance modes.
- **Do** preserve text labels, visible focus, native form semantics and independent content scrolling.
- **Do** keep the original folder art local, decorative, proportional and optional at constrained widths.
- **Do** retain crisp table rows, selected-state contrast and tabular numerals.

### Don't:

- **Don't** ship competitor screenshots, symbols or branding as product assets.
- **Don't** fabricate scan metrics or use celebratory deletion motion.
- **Don't** turn each filename into a decorative card or let art block the action.
- **Don't** remove gradients, rounded surfaces or dimensional art merely to satisfy the superseded Space Layers direction.

No material visual deficiency was identified against the pinned reference in the recorded finish review. Missing concept-roll seed is a manager-accepted process limit; the formal reviewer disposition remains `fix`. VoiceOver spoken output remains unverified under the accepted environment limit. This documentation does not assert an unconditional ship verdict; see [review disposition](.impeccable/review/verdict.md) and [native acceptance evidence](docs/specs/features/application-shell.md).
