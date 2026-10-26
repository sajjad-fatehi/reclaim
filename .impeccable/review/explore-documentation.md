# Explore design-system inheritance

This is an ordinary, code-led Operate extension. `DESIGN.md` and `.impeccable/design.json` remain unchanged; no new visual world or system-wide rule was introduced.

## Source evidence

Checked `PRODUCT.md`, `DESIGN.md`, `.impeccable/design.json`, `.impeccable/surfaces/application-shell.md`, `docs/design/warm-reference.md`, `src/shell/tokens.css`, `src/styles.css`, `src/shell/primitives.tsx`, and `src/explore/{Explore,ItemInspector,model,contract}`.

- Palette: Explore uses existing surface, secondary, accent, selection and selection-text roles. Runtime light/dark token overrides remain authoritative.
- Typography: platform system face and inherited body sizing remain; row paths use the existing label size (`0.75rem`), facts use tabular numerals, long identities wrap.
- Layout: flat semantic rows, distinct Open/Inspect buttons, wrapping breadcrumbs and toolbar; table viewport caps at `32rem`, with sticky column headers. Fifty-row pagination bounds mounted results. The shared inspector retains the existing inline/edge-sheet breakpoint (`1100px`).
- Shapes and depth: existing controls, notices, inspector and dialog are reused. Explore adds no color primitive, shadow, radius scale or raster asset.
- Named rules preserved: Warm Frame, Paired Theme and Quiet Depth. Existing motion/reduced-motion behavior is inherited through the shared inspector and controls; this is a source observation, not a rendered verification.

## Evidence limits and disposition

Formal finish-review disposition: **recapture**. No current Explore screenshots exist. Missing matrix: light/dark; compact `900×600`; `200%` text; reduced motion; native keyboard/VoiceOver/Finder handoff. CUA app attachment returned `cgWindowNotFound`; supported browser inventory was empty. The manager accepted these as explicit environment delivery limits. Source inspection and component tests do not certify rendered appearance, legibility, overflow, native assistive technology or Finder handoff.

Pre-existing sidecar staleness and detector advisories at `src/styles.css:103`, `:593` and `:798` were not repaired or canonized; they concern incumbent typography/radius records outside this extension. Product/surface statements that Explore is pending are historical status text, not new visual rules; the delivery owner updates authoritative feature status separately.
