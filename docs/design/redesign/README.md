# Redesign directions

[Design index](../index.md)

Three standalone mockups. Open the HTML files directly in a browser; there is no build step. Fonts for B and C load from Google Fonts and fall back to system fonts offline.

| Direction | File | Default theme |
| --- | --- | --- |
| A · Quiet Glass (native macOS) | [a-quiet-glass.html](a-quiet-glass.html) | System |
| B · Console (dense pro tool) | [b-console.html](b-console.html) | System |
| C · Atlas (visualization-led) | [c-atlas.html](c-atlas.html) | System |

Each file is a 1180×760 app window (design minimum is about 1100×720) with working navigation, a theme toggle (System → Light → Dark), a selectable Explore list or treemap with inspector, and a Review & Clean confirmation. Query parameters help review and screenshots: `?theme=light|dark&view=overview|explore|review`. A also accepts `&sheet=1`; B accepts `view=review&confirm=1` and `view=palette`.

Screenshots of every direction in light and dark are in [screenshots/](screenshots/).

## What stays the same in every direction

Product behavior, data contracts and safety rules do not change. All three mockups use the same synthetic data (shaped like `src/scan`, `src/explore`, `src/development`, `src/cleanup`) and keep these truths visible:

- **Partial coverage.** 3 of 5 locations are measured. Two need access and say so beside the numbers, in the shell status and in the coverage view. The volume is never presented as fully explained.
- **Scope versus volume.** 214.6 GB is measured in scope, 597.7 GB lies outside it and is hatched ("not measured"), 182.4 GB is free. Category shares are labelled as shares of the measured scope.
- **Unknown stays unknown.** "Unrecognized / Ownership unknown" is a hatched category. An unmeasured entry (`.env.local`, permission denied) is shown as "Not measured", never as zero or as a missing tile.
- **Consequences and evidence.** The inspector states owner, confidence, evidence, allocated and logical size, measurement time and an "If removed" sentence. It never says "safe to delete". Allocated size is not promised as freed space on APFS.
- **Restrictions.** A worktree with uncommitted changes, protected source and `.git`, and unknown-owner folders cannot be added to review and say why.
- **Review.** Each item shows its method and reversibility (Trash versus a non-force Docker prune). Estimated recovery reads "Unknown". Revalidation is stated. Initial focus is Cancel and the irreversible part needs an explicit acknowledgement before Remove enables.
- **Motion** is decorative or relational, never the only carrier of meaning. `prefers-reduced-motion: reduce` disables every animation and transition in all three files; end states render immediately.
- **Accessibility.** Visible 2–3 px focus ring, keyboard-operable rows and tiles, textual state labels beside every color, `aria-live` inspector, `<dialog>` with focus return.

Contrast was computed from the declared tokens (WCAG relative luminance): body text, secondary text, accent text, text on accent, warning and danger pairs, and dark tile text on every tile fill all reach at least 4.5:1 in both themes; C's saturated chart fills were checked at 3:1 or more against its light background (dark fills are bright on a dark canvas). A and B swatches were not measured as graphics and always sit beside text labels. Disabled controls are exempt. In A, category swatches on white are around 2–3:1; every swatch sits beside a text label and value, so color is never the only carrier. Spoken VoiceOver output and native WebView rendering were not verified.

## A · Quiet Glass (native macOS)

**Idea.** Disappear into macOS. A translucent source-list sidebar, a standard toolbar, inset grouped lists, a trailing inspector and a sheet for confirmation. Calm, familiar, lowest learning cost.

**Layout and IA changes**
- Sidebar keeps the five destinations but adds a persistent scan chip (partial scan, locations, time) at the foot so coverage is visible from every section. Review & Clean carries a count badge.
- Overview becomes a document: volume title, one capacity bar (measured / outside scope hatched / free), coverage banner, "Follow the space" grouped list, "Worth inspecting" cards.
- Explore is a Finder-style table with a path bar at the bottom and a permanent trailing inspector.
- Review is a list plus a macOS-style sheet that slides from the toolbar.

**Tokens**

| Token | Light | Dark |
| --- | --- | --- |
| Window / pane | `#FFFFFF` / `#F5F5F7` | `#242427` / `#1B1B1D` |
| Text / secondary | `#1D1D1F` / `#5B5B61` | `#F5F5F7` / `#A7A7AF` |
| Accent (fill / text) | `#0058D0` / `#0050BD` | `#0A60DC` / `#6CADFF` |
| Caution | `#6B4200` on `#FFF3D6` | `#F3D08C` on `#34290F` |
| Categories | deps `#3478F6`, build `#FF9500`, docker `#17A2B8`, stores `#A653D8`, editor `#2FB350`, worktrees `#F0365F`, logs `#8E8E93` | brighter variants of the same hues |
| Sidebar | `rgba(238,238,243,.66)` + 30 px blur | `rgba(44,44,50,.6)` + 30 px blur |

**Typography.** System font stack (SF Pro on macOS). 13 px body, 15 px section titles, 26 px bold large title, tabular numerals.

**Spacing and shape.** 4 pt grid. Radius 7–12 px. Hairline separators. Sidebar 232 px, inspector 320 px, toolbar 52 px.

**Iconography.** 16–18 px outlined strokes (1.6 px, round caps), SF Symbols feel. Colored rounded-square chips for categories.

**Motion principles.** Short, springy, physical. Capacity bar and meters grow from the leading edge (0.8 s, staggered 50 ms). Selected sidebar symbol bounces once. Category chip tilts and scales on hover while the matching segment in the capacity bar stays bright and the rest dim. Inspector content fades up on selection. Sheet drops in from the toolbar. Reduced motion: final states, no travel.

**Trade-offs.** + Feels first-party, most accessible by default, easiest to implement. − Least distinctive; the "ugly, boring" risk is smallest but so is the wow; expressive motion is limited to small flourishes.

## B · Console (dense pro tool)

**Idea.** A keyboard-first investigation instrument in the spirit of Linear and Raycast. Numbers, tables and shortcuts first, decoration minimal.

**Layout and IA changes**
- The sidebar becomes a 56 px icon rail with labelled tooltips and shortcut hints (`G O`, `G E`, `G D`, `G R`). A top bar holds a ⌘K command palette that searches scanned paths and actions, and a live scan status.
- Overview is a KPI strip plus a thin volume strip, then a ranked category table and a coverage table on the left and a "Needs attention" queue on the right (access required, dirty worktrees, persistent Docker data). Suggestions become an attention queue rather than cards.
- Explore puts the list on top and a docked three-column inspector below it (identity and path, ownership and size, consequence and action). Arrow keys move the selection and the inspector follows.
- Review & Clean is a right-hand drawer available from anywhere (rail badge). Confirmation happens inside the drawer footer, so context stays visible behind a scrim.

**Tokens**

| Token | Dark | Light |
| --- | --- | --- |
| Background / panel / raised | `#0A0B0E` / `#101217` / `#171A21` | `#F4F5F8` / `#FFFFFF` / `#F0F1F5` |
| Line | `#252936` | `#E0E3EA` |
| Text / secondary | `#E8EAF0` / `#9BA2B3` | `#14161B` / `#555B6A` |
| Accent (fill / text / soft) | `#5468F0` / `#9DB0FF` / `#1B2142` | `#3F51D8` / `#3446C9` / `#E8EBFD` |
| Warn / ok / bad | `#F3C76B` / `#6FD49B` / `#FF8C84` | `#8A5200` / `#146C43` / `#B3261E` |
| Categories | `#7B93FF #FFB340 #3CC7DD #C08CFF #4FD187 #FF6B9A #8E95A8` | `#4F6BFF #E08A00 #0F9FB5 #9B5DE5 #1F9D55 #E0366B #7B8294` |

**Typography.** Inter 12.5 px body, JetBrains Mono for every number, path and shortcut, uppercase 10.5–11 px micro labels. Dark is the primary theme.

**Spacing and shape.** 30 px table rows, 8–16 px panel padding, radius 5–8 px, one-pixel borders, almost no shadow except overlays.

**Iconography.** 15–18 px, thinner 1.35 px stroke with square caps. Functional, never decorative. Status uses text pills with an icon, not colored dots alone.

**Motion principles.** Fast and informational (120–220 ms, ease-out, no bounce). Rail indicator grows on arrival. Numbers count up once on load. Rows react at 80 ms. Drawer slides from its real edge. A pulsing dot marks a partial or live scan. Reduced motion removes the pulse, count-up and slides.

**Trade-offs.** + Highest information density, fastest for repeat use, and excellent for the "which exact path" question. − Least approachable for people unfamiliar with development artifacts, lowest delight, relies on mono text and shortcuts that need discoverability. The user asked for expressive icons and motion; B delivers them only in restrained form.

## C · Atlas (visualization-led)

**Idea.** Space is a shape you can read. A sunburst shows categories and their biggest items; a treemap shows what fills a folder. Floating glass panels, warm color, playful motion. Closest to the existing warm and CleanMyMac-style brief.

**Layout and IA changes**
- No sidebar. A floating pill navigation sits at the top, with Review & Clean carrying a count. The window is a canvas.
- Overview leads with a two-ring sunburst of the measured scope only (inner: categories, outer: largest items), a coverage ribbon beside it (measured / outside scope hatched / free) and a legend that doubles as keyboard-focusable list. The center label updates with the hovered or focused slice. The caption states the chart's denominator. Suggestions sit in a bottom row.
- Explore leads with a treemap with a breadcrumb pill trail, a Treemap/List toggle (the list is the accessible and precise equivalent) and a floating inspector whose header takes the tile's color. Unmeasured entries are listed under the chart as a dashed "Not measured" strip, never silently dropped. A "review tray" under the chart collects chips for selected items.
- Review & Clean is a centered dialog whose colored dots echo the tiles the items came from.

**Tokens**

| Token | Dark | Light |
| --- | --- | --- |
| Background | `#120E1C` plus violet and plum glows | `#FBF5EC` plus peach and lilac glows |
| Glass / edge | `rgba(255,255,255,.07)` / `rgba(255,255,255,.14)` | `rgba(255,255,255,.72)` / `rgba(60,40,90,.16)` |
| Text / secondary | `#F4F0FB` / `#BDB4D1` | `#241A33` / `#5D5270` |
| Accent (fill / text) | `#7A5CF0` / `#C0B0FF` | `#5B3FD6` / `#4A2FC0` |
| Tile fills (dark text `#1C1626`) | `#A3B2FF #FFBF80 #70D9EC #CDA4FF #86E3A5 #FF9DBB #C4BFD3` | pastel `#B2BFFF #FFCD96 #93E0EE #DCC2FF #A2E8BB #FFB6CD #D5D0DF` |
| Chart fills | same as tiles | saturated `#4A60E6 #C96A00 #0A8FA6 #8648DC #17924D #D82F60 #716A85` |

**Typography.** Bricolage Grotesque for display, numbers and titles; DM Sans for body. 13.5 px body, 30 px page title, 34 px inspector figure.

**Spacing and shape.** Generous, 12–26 px gutters, radius 14–26 px, pill controls, soft shadows, backdrop blur on glass.

**Iconography.** 18 px, 1.7 px round-cap strokes, plus colored tiles and chips as the main visual identifier. Room for original dimensional illustration in first-run and empty states.

**Motion principles.** Expressive but bounded. Slices sweep in from the center, staggered 35 ms (0.8 s). Hovering a slice pushes it outward and dims the rest while the center label, legend row and tile list stay synchronized. Treemap tiles pop in, lift on hover and animate position and size when a filter or drill changes the layout. Inspector slides from its edge. Tab pill squishes on arrival. Chips drop into the review tray and dialog. Deletion is never celebrated. Reduced motion: everything appears in its final position.

**Honesty rules specific to charts.** The sunburst draws only measured scope and says so. Outside-scope and access-required space appear in the coverage ribbon, hatched, not as slices. Treemap area is allocated size; very small entries get a minimum visible size (stated in the real implementation's tooltip copy) and unmeasured entries are listed outside the chart. A list view is always one click away.

**Trade-offs.** + Most memorable, best answer to "where did the space go?", best fit with the user's request for expressive icons and motion. − Highest build cost (SVG chart layout, treemap, hit testing, keyboard and VoiceOver equivalents), highest risk of implying completeness if the chart is misused, and accuracy limits (APFS clones, hardlinks, logical versus allocated) need careful caption design. Dense inspection of thousands of rows still wants a table.

## Comparison

| | A · Quiet Glass | B · Console | C · Atlas |
| --- | --- | --- | --- |
| Navigation | Translucent sidebar | Icon rail, ⌘K, shortcuts | Floating pill tabs |
| Overview hero | Capacity bar + grouped list | KPI strip + ranked table | Sunburst + coverage ribbon |
| Explore | Table + trailing inspector | Table + docked inspector | Treemap (or list) + floating inspector |
| Review | Sheet | Drawer | Centered dialog |
| Density | Medium | High | Low to medium |
| Expressiveness | Low | Low | High |
| Truthfulness risk | Low | Low | Medium (charts need discipline) |
| Build cost | Low | Medium | High |
| Native fit | Highest | Medium | Lowest |

## Recommendation

Choose **C · Atlas** as the visual direction, with two borrowings. It is the only direction that meaningfully answers "ugly" and the stated wish for expressive motion, and the spatial model fits the product question (where did space go, which path owns it). Keep **B's** table with arrow-key navigation as Explore's default dense list and its shortcut layer, and keep **A's** conservative confirmation behavior (Cancel focused, explicit acknowledgement, sheet-like dialog). If build cost or accessibility risk is the priority over delight, choose **A**; it is the safest path and still a clear improvement. **B** is best as a later "pro density" mode rather than the first impression.

## Chosen direction

Decided by the user on 2026-10-27: **C · Atlas**, with these borrowings.

- **From C:** visual language, palette, typography, motion, floating pill navigation, sunburst Overview, and treemap Explore with a floating inspector. Hatched outside-scope and unrecognized areas stay in every chart.
- **From B:** the **Needs attention** panel on Overview (access required, worktrees with uncommitted changes, Docker volumes, large recognized items), which replaces C's bottom cards. Also B's dense table with arrow-key navigation as Explore's list mode, and its keyboard shortcut layer.
- **From A:** conservative cleanup confirmation, with Cancel focused initially, an explicit acknowledgement for irreversible removal and a sheet-like dialog.
- **Not chosen:** A's translucent sidebar shell and B's icon rail and monospace console styling.

The mockups use different sample totals (B shows 787.2 GB used and A/C show 812.3 GB). Treat that as a mockup artifact; real values come from the scan session.

## Implementation notes for the next task

- Tokens above map to CSS custom properties in `src/shell/tokens.css`; chart fills and tile fills are separate token sets so text contrast and graphic contrast can be tuned independently.
- The mockups use inline SVG symbols. The product icon family decision (Phosphor versus another) is unchanged by this task.
- Nothing in `src/`, `src-tauri/` or `crates/` was changed.
