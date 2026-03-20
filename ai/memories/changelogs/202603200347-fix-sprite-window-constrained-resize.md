## 2026-03-20 03:47: fix: constrained sprite window resize update

**What changed:**
- Enabled constrained programmatic resizing for the main sprite window in the Tauri app.
- Switched the main window configuration to `resizable: true` so platform window managers can honor resize requests more reliably.
- Updated the Rust `resize_sprite_window` command to:
  - enable resizing on the window,
  - apply tight min/max size constraints using Tauri logical pixel units,
  - keep position adjustments in sync while the sprite window expands or shrinks.
- Added required window permissions for:
  - starting resize drags,
  - setting size,
  - setting size constraints,
  - toggling resizable state.
- Increased mini chat layout dimensions so the sprite window has enough room for:
  - the open mini chat tray,
  - compact reply bubbles,
  - expanded reading mode replies.
- Updated sprite layout tests to match the new window width, height, and padding calculations.
- Kept panel resize handles and widened expanded mini chat bubble styling aligned with the new resizing behavior.

**Why:**
- Linux and Wayland compositors can behave inconsistently when resizing undecorated non-resizable windows.
- Using a resizable window with tight constraints is a more robust cross-platform strategy for automatic sprite window growth and shrink behavior.
- The larger mini chat dimensions prevent clipping and keep the sprite centered while preserving the intended visual layout.

**Files affected:**
- `apps/desktop-tauri/src-tauri/src/lib.rs`
- `apps/desktop-tauri/src-tauri/tauri.conf.json`
- `apps/desktop-tauri/src-tauri/capabilities/default.json`
- `apps/desktop-tauri/src-tauri/gen/schemas/capabilities.json`
- `apps/desktop-ui/src/lib/sprite-bubble-layout.ts`
- `apps/desktop-ui/src/lib/sprite-bubble-layout.test.ts`
- `apps/desktop-ui/src/components/panels/PanelShell.tsx`
- `apps/desktop-ui/src/components/sprite/SpriteMiniChatBubble.tsx`
- `apps/desktop-ui/src/views/SpriteView.tsx`

**Verification:**
- `cargo check -p peekoo-desktop-tauri`
- `bun x tsc --noEmit -p apps/desktop-ui/tsconfig.json`
- `bun run --cwd apps/desktop-ui build`
- `bun test apps/desktop-ui/src/lib/sprite-bubble-layout.test.ts`
