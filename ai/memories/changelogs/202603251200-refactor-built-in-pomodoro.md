## 2026-03-25 12:00: refactor: move pomodoro to built-in runtime

**What changed:**
- Added `peekoo-pomodoro-domain` for pomodoro state, settings, and cycle history rules
- Added `peekoo-pomodoro-app` for persistence, scheduler recovery, notifications, badges, and mood reactions
- Added SQLite migration `0010_pomodoro_runtime.sql` for active pomodoro state and cycle history
- Wired `peekoo-agent-app`, Tauri commands, and desktop UI pomodoro flows to the new built-in service
- Removed the legacy `plugins/pomodoro` plugin directory after built-in parity was verified

**Why:**
- The previous pomodoro flow was split across plugin and built-in paths, which caused contract drift and fragile behavior
- The new built-in design gives pomodoro a single source of truth with restart recovery and persisted history

**Files affected:**
- `Cargo.toml`
- `crates/peekoo-pomodoro-domain/`
- `crates/peekoo-pomodoro-app/`
- `crates/persistence-sqlite/migrations/0010_pomodoro_runtime.sql`
- `crates/persistence-sqlite/src/lib.rs`
- `crates/peekoo-agent-app/src/application.rs`
- `crates/peekoo-agent-app/src/lib.rs`
- `crates/peekoo-agent-app/src/productivity.rs`
- `crates/peekoo-agent-app/src/settings/store.rs`
- `apps/desktop-tauri/src-tauri/src/lib.rs`
- `apps/desktop-ui/src/features/pomodoro/`
- `apps/desktop-ui/src/hooks/use-pomodoro-watcher.ts`
- `apps/desktop-ui/src/components/sprite/SpritePeekBadge.tsx`
- `apps/desktop-ui/src/components/sprite/SpriteBubble.tsx`
