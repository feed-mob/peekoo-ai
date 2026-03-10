## 2026-03-10 17:30: feat: generic peek badge system for sprite status display

**What changed:**
- Added `PeekBadgeService` to `peekoo-notifications` crate: a `Mutex<HashMap>`-based service that collects badge items per-plugin with dirty-flag change detection
- Added `peekoo_set_peek_badge` host function to `peekoo-plugin-host` so WASM plugins can push status badge items to the sprite
- Wired `PeekBadgeService` through `PluginRegistry`, `AgentApplication`, and the Tauri background flush loop (250ms interval)
- Added `flush_peek_badges()` in the Tauri layer that emits `sprite:peek-badges` events to the main window when badge data changes
- Updated the `health-reminders` WASM plugin with `push_peek_badges()` that converts active reminder states into `PeekBadgeItem` values, called on init, event handling, configure, and dismiss
- Created frontend types (`peek-badge.ts`), `usePeekBadge` hook (event listener + 1s countdown tick + 5s rotation), and `SpritePeekBadge` component (collapsed pill / expanded stacked list)
- Updated `sprite-bubble-layout.ts` with badge-aware window sizing (`peekBadgeExtraHeight`)
- Wired `SpritePeekBadge` into `SpriteView.tsx` with visibility rules: hidden during action menu, hidden during notification bubble, auto-collapse on bubble fire
- Extended `sprite-bubble.test.ts` with badge layout tests and `PeekBadgeItemSchema` validation tests

**Also in this session (prior commits):**
- Fixed health reminders panel countdown: added 30s backend polling, 1s local countdown ticks, "Due now" state at zero, and immediate backend refresh when a reminder becomes due
- Added `health-reminders-panel.test.js` regression tests for all countdown behaviors

**Why:**
- The sprite had no persistent status indicators -- users could only see reminder countdowns by opening the health panel
- The peek badge system is generic: any plugin can call `peekoo_set_peek_badge` to contribute status items, making it extensible for pomodoro timers, calendar events, etc.
- Push-based architecture (vs polling) ensures badges update within ~250ms of a plugin state change

**Files affected:**
- `crates/peekoo-notifications/src/lib.rs`
- `crates/peekoo-notifications/src/peek_badge.rs` (new)
- `crates/peekoo-plugin-host/src/host_functions.rs`
- `crates/peekoo-plugin-host/src/registry.rs`
- `crates/peekoo-agent-app/src/application.rs`
- `crates/peekoo-agent-app/src/lib.rs`
- `crates/peekoo-plugin-store/src/lib.rs` (test helpers)
- `apps/desktop-tauri/src-tauri/src/lib.rs`
- `apps/desktop-ui/src/types/peek-badge.ts` (new)
- `apps/desktop-ui/src/hooks/use-peek-badge.ts` (new)
- `apps/desktop-ui/src/components/sprite/SpritePeekBadge.tsx` (new)
- `apps/desktop-ui/src/lib/sprite-bubble-layout.ts`
- `apps/desktop-ui/src/views/SpriteView.tsx`
- `apps/desktop-ui/tests/sprite-bubble.test.ts`
- `apps/desktop-ui/tests/health-reminders-panel.test.js` (new)
- `plugins/health-reminders/src/lib.rs`
- `plugins/health-reminders/ui/panel.js`
