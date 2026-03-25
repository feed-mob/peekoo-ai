# Plan: Built-in Pomodoro Refactor

## Overview
Replace the mixed plugin/built-in pomodoro implementation with a fully built-in feature backed by dedicated Rust crates. The new design persists the active timer and settings, records completed and cancelled focus/break cycles, and routes countdown, notifications, badges, and mood updates through built-in services instead of the plugin host.

## Goals
- [x] Remove pomodoro's runtime dependency on the plugin system
- [x] Add dedicated domain and app crates for pomodoro
- [x] Persist current timer state and settings across app restarts
- [x] Persist cycle history for completed and cancelled work/break sessions
- [x] Route pomodoro countdown, badges, notifications, and mood reactions through built-in services
- [x] Switch the desktop UI from `plugin_call_tool` to dedicated built-in pomodoro commands
- [x] Cover the new behavior with domain, app, and UI client tests

## Design

### Approach
Create a dedicated pomodoro subsystem with a pure domain crate and an application crate. The domain crate owns modes, states, settings, history record types, and transition rules. The app crate owns persistence, scheduler integration, recovery, side effects, and DTOs. `peekoo-agent-app` becomes a composition root that wires the pomodoro app service into the existing desktop runtime.

### Components
- `crates/peekoo-pomodoro-domain` - Pure pomodoro state machine, validation, and history types
- `crates/peekoo-pomodoro-app` - Runtime orchestration, SQLite persistence, scheduler recovery, and DTO shaping
- `crates/persistence-sqlite` - Embedded migration for pomodoro runtime state and cycle history tables
- `crates/peekoo-agent-app` - Application composition and Tauri-facing pomodoro methods
- `apps/desktop-tauri/src-tauri` - Dedicated built-in pomodoro commands
- `apps/desktop-ui/src/features/pomodoro` - Typed pomodoro client and panel updates
- `apps/desktop-ui/src/components/sprite` and `apps/desktop-ui/src/hooks` - Built-in command usage for badge, bubble, and memo watcher flows

## Implementation Steps

1. **Create the new pomodoro crates**
   - Add `peekoo-pomodoro-domain` to hold the pomodoro model and invariants
   - Add `peekoo-pomodoro-app` to manage persistence, scheduling, status projection, and side effects
   - Add domain and app-level tests first

2. **Add persistence and recovery support**
   - Add a new SQLite migration for active pomodoro state and cycle history
   - Ensure migrations run during app startup
   - Restore pending state on startup and re-arm or reconcile the countdown

3. **Wire pomodoro into the app layer**
   - Compose the new pomodoro app service inside `peekoo-agent-app`
   - Expose dedicated methods for status, settings, start, pause, resume, finish, switch mode, and history
   - Retire the old in-memory pomodoro implementation from `ProductivityService`

4. **Switch desktop transport and frontend consumers**
   - Replace plugin-based pomodoro calls with built-in Tauri commands
   - Update the panel, badge controls, bubble dismiss, and memo watcher to use the typed client
   - Remove redundant frontend-only pet reaction side effects once the backend owns them

5. **Verify parity and clean up**
   - Run targeted Rust and UI tests
   - Build the desktop UI to catch type regressions
   - Remove the legacy pomodoro plugin once built-in parity is confirmed

## Status
- Completed the new pomodoro domain and app crates and wired them into `peekoo-agent-app`
- Added SQLite migration `0010_pomodoro_runtime.sql` for runtime state and history
- Switched the desktop UI pomodoro flows to dedicated built-in commands
- Removed the legacy `plugins/pomodoro` implementation from the repository
- Verified with targeted Rust tests, Tauri crate check, and desktop UI production build

## Files to Modify/Create
- `Cargo.toml` - Add new workspace crates
- `crates/peekoo-pomodoro-domain/*` - New domain crate
- `crates/peekoo-pomodoro-app/*` - New app crate
- `crates/persistence-sqlite/migrations/0010_pomodoro_runtime.sql` - New migration
- `crates/persistence-sqlite/src/lib.rs` - Export migration constant and tests
- `crates/peekoo-agent-app/src/application.rs` - Compose built-in pomodoro service
- `crates/peekoo-agent-app/src/lib.rs` - Re-export pomodoro DTOs
- `crates/peekoo-agent-app/tests/productivity_service.rs` - Remove old pomodoro coverage or move to new tests
- `apps/desktop-tauri/src-tauri/src/lib.rs` - Built-in pomodoro command handlers
- `apps/desktop-ui/src/features/pomodoro/*` - Typed client and panel updates
- `apps/desktop-ui/src/hooks/use-pomodoro-watcher.ts` - Built-in status polling
- `apps/desktop-ui/src/components/sprite/SpritePeekBadge.tsx` - Built-in pause/resume handling
- `apps/desktop-ui/src/components/sprite/SpriteBubble.tsx` - Built-in dismiss handling

## Testing Strategy
- Domain tests for valid and invalid state transitions, settings updates, and counter changes
- App tests for persistence, resume-after-restart behavior, completion logging, cancellation logging, and DTO projection
- Migration tests for new pomodoro tables
- Frontend client tests for command invocation behavior
- `cargo test` for new crates and app integration points
- `bun run build` in `apps/desktop-ui` for frontend type validation

## Open Questions
- None.
