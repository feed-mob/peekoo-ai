# 2026-02-15 02:07: Desktop Pet Multi-Window Redesign

## Summary
Converted the desktop UI from a single large in-window dashboard into a desktop-pet architecture:
- A small transparent sprite window (`200x250`) that stays on top
- On-demand feature windows for Chat, Tasks, and Pomodoro
- Label-based window routing in React
- Removal of Three.js background stack for performance and transparency
- Fix for Vite stale dependency cache after package removal

## Why This Change
- The previous `1000x700` transparent app behaved like a regular app window, not a desktop companion
- Floating panels, dock, and full-screen effects consumed space and clicks meant for desktop usage
- Three.js background added unnecessary bundle/runtime cost for a tiny pet surface

## Architecture Shift

### Before
- Single Tauri window
- Centered sprite + floating panels + panel dock in one React tree
- WebGL particle background rendered full-screen

### After
- Sprite-only main window (`main`) with transparent background
- Separate Tauri `WebviewWindow`s for:
  - `panel-chat`
  - `panel-tasks`
  - `panel-pomodoro`
- Radial sprite action menu opens/closes panel windows
- Cross-window event channel for sprite reactions (`pet:react`)

## Tauri Changes

### `apps/desktop-tauri/src-tauri/tauri.conf.json`
- Main window changed to desktop-pet profile:
  - `label: "main"`
  - `width: 200`, `height: 250`
  - `resizable: false`
  - `decorations: false`
  - `transparent: true`
  - `alwaysOnTop: true`
  - `skipTaskbar: true`
- Removed previous large-window constraints (`minWidth`, `minHeight`)

### `apps/desktop-tauri/src-tauri/capabilities/default.json`
- Expanded capability scope from `main` only to `main` + `panel-*`
- Added window management permissions:
  - `core:window:default`
  - `core:window:allow-start-dragging`
  - `core:window:allow-close`
  - `core:window:allow-set-size`
  - `core:window:allow-set-position`
  - `core:window:allow-set-focus`
  - `core:window:allow-center`
- Added Webview and event permissions:
  - `core:webview:allow-create-webview-window`
  - `core:event:default`
  - `core:event:allow-emit-to`
  - `core:event:allow-listen`

## Frontend Changes

### New Routing + Views
- `apps/desktop-ui/src/main.tsx`
  - Now reads current window label via `getCurrentWebviewWindow().label`
  - Renders a label-resolved view instead of `App.tsx`
- Added `apps/desktop-ui/src/routing/resolve-view.tsx`
  - Lazy-loads view by label with `Suspense`
- Added view files:
  - `apps/desktop-ui/src/views/SpriteView.tsx`
  - `apps/desktop-ui/src/views/ChatView.tsx`
  - `apps/desktop-ui/src/views/TasksView.tsx`
  - `apps/desktop-ui/src/views/PomodoroView.tsx`

### New Window/Events Types
- `apps/desktop-ui/src/types/window.ts`
  - `WindowLabelSchema`, `PanelLabelSchema`
  - `PANEL_WINDOW_CONFIGS` for per-panel window sizing/title
- `apps/desktop-ui/src/types/pet-event.ts`
  - `PetReactionTriggerSchema`
  - `PetReactionEventSchema`

### New Hooks
- `apps/desktop-ui/src/hooks/use-panel-windows.ts`
  - Creates/focuses/closes panel `WebviewWindow`s
  - Tracks panel open state
  - Handles temporary sprite window expand/shrink for menu
- `apps/desktop-ui/src/hooks/use-sprite-reactions.ts`
  - Listens for `pet:react` events with Zod payload validation

### New Components
- `apps/desktop-ui/src/components/panels/PanelShell.tsx`
  - Shared panel frame with drag region and close action
  - Emits `pet:react` on close
- `apps/desktop-ui/src/components/sprite/SpriteActionMenu.tsx`
  - Animated radial menu for Chat/Tasks/Pomodoro
  - Shows open-state styling per panel

### Updated Components/Styles
- `apps/desktop-ui/src/components/sprite/Sprite.tsx`
  - Removed speech bubble, kept sprite-only rendering
- `apps/desktop-ui/src/index.css`
  - `body` background changed to `transparent`

## Dependency and Cleanup Changes

### Removed Dependencies
- `@react-three/fiber`
- `@react-three/drei`
- `three`
- `@types/three`

### Removed Files
- `apps/desktop-ui/src/App.tsx`
- `apps/desktop-ui/src/components/WindowDragBar.tsx`
- `apps/desktop-ui/src/components/panels/FloatingPanel.tsx`
- `apps/desktop-ui/src/components/panels/PanelHeader.tsx`
- `apps/desktop-ui/src/components/panels/PanelDock.tsx`
- `apps/desktop-ui/src/hooks/use-panel-state.ts`
- `apps/desktop-ui/src/components/background/ParticleBackground.tsx`
- `apps/desktop-ui/src/components/background/StarField.tsx`
- `apps/desktop-ui/src/components/background/AmbientGlow.tsx`

## Post-Change Issue and Fix

### Issue
Vite failed with:
`ENOENT ... node_modules/@react-three/fiber/dist/react-three-fiber.esm.js`

### Cause
Stale Vite optimized dependency cache still referenced removed `@react-three/*` packages.

### Fix
Cleared caches and forced re-optimize:
```bash
rm -rf apps/desktop-ui/node_modules/.vite apps/desktop-ui/node_modules/.cache/vite
bun run dev --force
```

## Verification
- `bunx tsc --noEmit` passes
- `bun run build` passes
- Build output confirms per-view code splitting (`SpriteView`, `ChatView`, `TasksView`, `PomodoroView` chunks)

## Result
Peekoo now behaves like a desktop companion instead of a fullscreen widget shell: lightweight sprite presence by default, with feature windows launched only when requested.
