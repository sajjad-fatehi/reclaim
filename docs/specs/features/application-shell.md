# Application shell and appearance

Follow [shared requirements](index.md), [layouts](../../design/layouts.md), [design system](../../design/design-system.md) and [icons/motion](../../design/icons-and-motion.md).

## Outcome and ownership

Replace the scaffold-only composition with the working desktop shell and reusable presentation foundation. This can proceed independently of scanner implementation because it owns no scan or artifact contract. Future sections must be honestly identified as not yet implemented until their own tasks land.

## Required behavior

- Provide Overview, Explore, Development and Review & Clean navigation, Settings at the bottom, a volume/scan context region, and titlebar-safe desktop controls.
- Preserve per-view navigation/scroll state where applicable. Define stable route/view composition points and shared presentation primitives for toolbars, tables, notices, dialogs, inspectors and empty states. Do not create a universal component framework or duplicate data stores.
- Implement semantic Space Layers tokens in one frontend source, consistent typography and icon family, light/dark/system appearance and a locally persisted appearance preference. Validate persisted values; recover safely from corrupt configuration without overwriting unrelated settings.
- Deliver authored navigation, selection and panel motion that follows the documented vocabulary, can be interrupted, respects reduced motion and never delays input/focus. No synthetic scanning or success feedback.
- Respect macOS window controls and keyboard conventions. Adapt to the documented desktop frames, compact windows and 200% text without hiding actions or content. Do not introduce mobile navigation.
- Use semantic controls, keyboard-visible focus and accessible names. Keep future data/error states composable, rather than embedding mock application results into production.

## Acceptance and proof

- All destinations are keyboard-reachable, active state is clear and navigation does not destroy existing view state. Unimplemented sections never show fake results or dead cleanup controls.
- System/light/dark choice survives restart, follows OS changes in System mode and fails safely on invalid preferences. Theme changes affect all shared roles consistently.
- Capture and inspect light/dark desktop and compact/text-enlarged views; validate focus/dialog/reduced-motion behavior. Use focused tests for preference/navigation behavior and required checks.
- Apply ui-ux-pro-max, impeccable Operate guidance, engineering-skills:senior-frontend and make-interfaces-feel-better. Keep review bounded; include the user's existing Space Layers and icon/motion commitments. Record the built visual system through impeccable's applicable documentation workflow; do not overwrite product truth or invent a new visual direction.

## Implementation acceptance record

The shell implementation uses mounted, independently scrollable view slots for the five destinations. Navigation is local presentation state; consumers own their domain/session state. A dedicated `reclaim.appearance` localStorage key accepts only `system`, `light`, or `dark`; missing, malformed or inaccessible storage falls back to System without touching other keys. Failed writes keep the selected appearance for this session and show a useful notice. System changes remain live.

Native decorated titlebar remains outside web content. Shared native-semantic presentation primitives cover toolbar, table, notice, empty state, modal dialog and responsive inspector. No scanner/cleanup actions or result metrics ship in placeholders. Development-only labelled fixtures exercise composition, selection and overlays; production excludes them. Tests cover metadata, navigation state retention, appearance validation/storage failure/system changes and overlay semantics. Native QA covers both themes, compact/text enlargement, keyboard focus and reduced motion; report VoiceOver and platform limits explicitly.

### Consumer composition contract

- `src/shell/AppShell.tsx`: `AppShell({ connectionStatus, views?, context? })`; destination IDs are `overview | explore | development | review | settings` (`Destination` type). `views` is a partial record of React nodes. The four feature slots replace honest placeholders. The Settings slot follows built-in Appearance and replaces the pending scan-settings message. `context` replaces the header's unselected-volume / not-started region. Pass real scan context only from its owner.
- All slots mount once and remain mounted behind `hidden`; their local input/selection state and scroll containers survive navigation. Hidden views leave the accessibility/focus tree. Consumers own data/session lifecycles; keep stable component identities and never start scans from mounting a slot. Navigation defaults to Overview and is not persisted.
- `src/shell/primitives.tsx`: `Toolbar({ label, children })`, `Notice({ children })`, `EmptyState({ icon, title, children })`, `ResultTable({ caption, children })`, `Dialog({ title, open, onClose, children })`, `Inspector` with the same panel props. Consumers provide semantic table sections/headers/rows and real state/actions; primitives do not infer eligibility or operate on files. `.numeric` aligns measured columns; checkbox-checked rows receive selection emphasis. `Notice` deliberately has no live role: consumers choose live announcements only for meaningful updates.
- Dialog is controlled, uses native `showModal()` and browser Escape/focus containment/return. Close control receives initial focus. Consumer may put explicit safe confirmation controls in children; no cleanup policy lives here. Inspector is a nonmodal right column above 1100px, with close-focus/Escape/focus-return; at compact widths it uses a modal sheet. Keep Inspector inside `.workspace-split` next to main content. Compact transitions remount panel content; durable item state belongs to consumer.
- `src/shell/appearance.ts`: isolated key `reclaim.appearance`, plain validated enum string; no filesystem/API permission. OS media changes apply only in System. Other-window preference updates synchronize through `storage`. Failure to read falls back safely; failure to write shows a session-only notice. No unrelated key is written or erased.
- `src/shell/tokens.css` owns semantic roles, typography baseline, spacing, radii and motion timing/easing. Components consume roles, not screen-specific colors. `data-theme` on document root is resolved appearance; `data-appearance` records preference. Motion is decorative: no animation event grants state or delays focus/input. No first-render entrance; later navigation/panel entrances cancel on interruption or reduced-motion change. Unsupported WAAPI degrades to immediate content.
- Development fixture: `npm run dev:web`, then `http://localhost:1420/?fixture=shell`. Dynamic import is guarded by `import.meta.env.DEV`; production excludes fixture and synthetic data. No native permissions are added. Native decorated titlebar stays separate; initial window is 1180×760, existing 360px minimum retained with desktop rail/scrolling.
