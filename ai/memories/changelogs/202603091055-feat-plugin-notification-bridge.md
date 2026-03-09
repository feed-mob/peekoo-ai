## 2026-03-09 feat: plugin notification bridge and sprite speech bubble

**What changed:**
- Added a sandbox bridge so plugin panel iframes can call Tauri commands through `window.__TAURI__.core.invoke`
- Routed plugin-emitted notification events through the app layer into Tauri system notifications, with a Linux `notify-send` fallback
- Added speech bubble that appears above the sprite when plugins emit notifications
- Window expands upward (bubble above sprite) and auto-shrinks after 5s dismiss
- Rust command `resize_sprite_window` bypasses `resizable:false` JS restriction for programmatic window sizing
- Bubble uses glass styling with downward tail pointing to sprite
- Triggers 'reminder' animation while bubble is visible

**Why:**
- Plugin panel UIs were failing because sandboxed iframes did not have direct Tauri access
- Notification requests from plugins were queued but never surfaced to the operating system
- Desktop pets should show visual feedback when plugins send notifications
- Window must stay floating (resizable:false) but still support programmatic resize

**Files affected:**
- `apps/desktop-ui/src/views/PluginPanelView.tsx`
- `apps/desktop-ui/src/views/SpriteView.tsx`
- `apps/desktop-ui/src/lib/plugin-panel-bridge.ts`
- `apps/desktop-ui/src/lib/sprite-bubble-layout.ts`
- `apps/desktop-ui/src/components/sprite/SpriteBubble.tsx`
- `apps/desktop-ui/src/hooks/use-sprite-bubble.ts`
- `apps/desktop-ui/src/hooks/use-panel-windows.ts`
- `apps/desktop-ui/src/types/sprite-bubble.ts`
- `apps/desktop-ui/tests/plugin-panel-bridge.test.ts`
- `apps/desktop-ui/tests/sprite-bubble.test.ts`
- `apps/desktop-tauri/src-tauri/src/lib.rs`
- `apps/desktop-tauri/src-tauri/Cargo.toml`
- `apps/desktop-tauri/src-tauri/capabilities/default.json`
- `crates/peekoo-agent-app/src/application.rs`
- `crates/peekoo-agent-app/src/plugin.rs`
