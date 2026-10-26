# Icons and motion

[Design index](index.md)

## Intent and status

The user wants a visually engaging interface with icons, animations, transitions and motion. This is an explicit design preference. The proposed choreography below makes Space Layers feel responsive and authored; it is not implemented or performance-tested yet.

Motion should acknowledge actions, explain relationships and add character. A professional operational tool can feel lively without making inspection slower or using deletion as a celebration.

## Icon language

- Proposed family: Phosphor for React. Confirm coverage and licensing at implementation; do not install it during documentation work.
- Keep one family with a consistent regular stroke. A filled counterpart may identify the active navigation destination; do not mix unrelated visual styles.
- Use icons in navigation, artifact/type identification, action controls, scan status and empty states. Pair unfamiliar meanings with English labels.
- Proposed navigation metaphors: overview/volume, folders for Explore, code/project for Development, and checklist for Review & Clean. The cleanup section is a review destination, not a trash-can shortcut.
- Standard 16/20 px icons serve dense UI; selected empty states may use a composed 32–48 px treatment. Keep icon size separate from pointer hit area.
- Status icons must represent actual state. A checkmark means an operation succeeded, not that a finding is safe to delete.
- Meaningful icon-only controls have accessible names and tooltips; decorative icons are hidden from assistive technology. Emoji are not structural icons.
- Use verified official assets for tool brands only when needed; generic artifact icons are preferable to invented logos.

## Motion vocabulary

| Moment | Proposed behavior | Reduced-motion equivalent |
| --- | --- | --- |
| Hover and press | Immediate tonal response; a small pressed-state transform on suitable buttons | Color/opacity feedback without movement |
| Navigation | Selected indicator moves to the destination; content changes with a short transition, preserving remembered position | Indicator changes immediately; content appears without travel |
| Inspector | Panel enters from its actual edge; selected row stays anchored | Panel appears with no slide; focus behavior is unchanged |
| Usage selection | Segment and matching row gain coordinated emphasis, followed by inspector content | Immediate synchronized selection and textual details |
| Add to review | Row control acknowledges addition; persistent review count updates once | Text/state/count update without travel |
| Scan active | A restrained activity glyph or local progress treatment accompanies real stage/location text | Static activity icon and explicit Scanning text; measured updates remain visible |
| Scan complete | Status icon resolves once and the summary transitions to its completed state | Immediate completed label and icon |
| Measurement refresh | Changed proportions transition together after a bounded update, without constantly shuffling rows | Immediate proportions; stable rows and refresh notice |
| Operation outcome | Item status changes to the real outcome; errors remain available for inspection | Immediate icon/text outcome; same error details |
| Empty-to-results | One short, bounded group entrance where it helps establish the result list | Results appear immediately in reading order |

## Signature sequence: follow the space

Selecting a usage layer connects that segment to its ranked category row and inspector. Coordinated emphasis makes the ownership relationship visible; the row and detail remain independently usable with keyboard or assistive technology. Only a deliberate navigation/filter action changes scope. Animation itself never navigates, changes selection or grants cleanup consent.

Use a short coordinated sequence rather than scattering unrelated effects across every row. Do not animate every file arrival during a scan. New measurements must not imply a complete scan or a known denominator.

## Timing and interruption

- Feedback: 100–150 ms; state changes: 160–240 ms; panels/view continuity: 220–320 ms. These are proposed starting ranges, not mandatory delays.
- Favor natural deceleration. Keep exits faster than entrances; avoid spring overshoot for precise data and destructive actions.
- A bounded initial list entrance must finish within about 300 ms; do not stagger thousands of findings.
- Repeated interactions interrupt or reverse the current transition from its current visual state. Never queue a chain of animations behind the user.
- Commit the actual state promptly; animation must not delay input, focus, cancellation, an error message or confirmation. Do not tie correctness to animation-end events.
- Preserve table readability and stable hit targets. No hover lift on data rows, layout jumps, theatrical size counters or particle/confetti cleanup rewards.

## Runtime and accessibility

Use CSS for straightforward states and the Web Animations API or an existing suitable library for interruptible coordination. Add a motion dependency only when the actual interaction warrants it. Respect the target macOS WebView's feature support and provide ordinary DOM/CSS fallbacks.

Prefer bounded transform/opacity effects for movement; measure other properties before adopting them. Avoid repeatedly animating layout across long tables. Stop activity loops when the window is hidden or the activity ends; hidden scans continue to have an accurate state when the window returns.

Honor prefers-reduced-motion, including changes while the app is running. Remove nonessential spatial movement and loops while preserving textual and static state feedback. Do not require sound, motion or color to understand an operation. Avoid flashes and automatic focus moves caused by animation.

## Future implementation proof

Review keyboard and pointer paths, repeated/rapid switching, cancellation during motion, long lists, both themes, reduced motion, background/foreground transitions and VoiceOver in the macOS WebView. Inspect actual responsiveness on representative hardware; no frame-rate guarantee is made here.

This document does not add a prototype, icon package, motion library or running UI.
