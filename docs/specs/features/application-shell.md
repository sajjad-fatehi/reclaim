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
