## 2026-03-13 11:00: fix: peek badge not showing on startup and off-center positioning

**What changed:**
- Added `ui_ready` gate to `PeekBadgeService`: a `ui_ready: bool` flag (default `false`) that prevents `take_if_changed()` from consuming badge data before the frontend has registered its event listener
- Added `mark_ui_ready()` method on `PeekBadgeService` that flips the flag and re-marks buffered badges as dirty so they flush on the next tick
- Exposed `mark_ui_ready()` through `AgentApplication` and added a `ui_ready` Tauri command
- Frontend `usePeekBadge` hook now calls `invoke("ui_ready")` immediately after registering the `sprite:peek-badges` listener, unblocking backend badge emission
- Replaced hardcoded `left: BADGE_LEFT` pixel positioning on `SpritePeekBadge` with `left: "50%"` + `marginLeft: -(BADGE_WIDTH / 2)` for robust centering regardless of actual container width; removed unused `SPRITE_WIDTH` import

**Why:**
- Race condition at startup: `plugin_init()` pushed badges during `AgentApplication::new()`, the background flush loop emitted `sprite:peek-badges` ~250ms later, but the React frontend hadn't mounted its event listener yet. The event was lost and the dirty flag cleared, so badges never appeared until the plugin was toggled off/on.
- The `ui_ready` gate ensures badge data is retained until the frontend signals readiness, then flushed immediately.
- The badge centering used a fixed pixel offset calculated from `SPRITE_WIDTH`, which could appear off-center depending on actual rendered window dimensions or display scaling.

**Files affected:**
- `crates/peekoo-notifications/src/peek_badge.rs` - Added `ui_ready` field, `mark_ui_ready()`, gated `take_if_changed()`
- `crates/peekoo-agent-app/src/application.rs` - Exposed `mark_ui_ready()` delegate
- `apps/desktop-tauri/src-tauri/src/lib.rs` - Added `ui_ready` Tauri command, registered in `generate_handler!`
- `apps/desktop-ui/src/hooks/use-peek-badge.ts` - Added `invoke("ui_ready")` after listener registration
- `apps/desktop-ui/src/components/sprite/SpritePeekBadge.tsx` - CSS centering fix, removed `SPRITE_WIDTH` import
