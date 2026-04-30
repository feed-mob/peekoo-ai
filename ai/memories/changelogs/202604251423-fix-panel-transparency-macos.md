## 2026-04-25 14:23: fix: Stabilize macOS panel transparency

**What changed:**
- Added a macOS-specific panel transparency helper that reapplies transparent window background state after panel creation, moves, focus changes, resizes, and DOM updates.
- Updated panel window creation to set an explicit transparent background color and native macOS window effects for panel windows.
- Switched panel shell styling on macOS away from the heavier CSS backdrop blur path to a more stable translucent shell while keeping the existing look on other platforms.
- Extended panel window tests to cover the new transparent background option.

**Why:**
- Panel windows on macOS were temporarily rendering as opaque after initial open, after dragging, and after in-panel UI changes before eventually becoming transparent again.

**Files affected:**
- `apps/desktop-ui/src/lib/window-transparency.ts`
- `apps/desktop-ui/src/hooks/use-panel-windows.ts`
- `apps/desktop-ui/src/hooks/use-pomodoro-watcher.ts`
- `apps/desktop-ui/src/main.tsx`
- `apps/desktop-ui/src/components/panels/PanelShell.tsx`
- `apps/desktop-ui/src/views/PomodoroMemoView.tsx`
- `apps/desktop-ui/src/hooks/use-panel-windows-open.test.ts`
- `apps/desktop-ui/src/hooks/use-panel-windows.test.ts`
- `apps/desktop-tauri/src-tauri/capabilities/default.json`
