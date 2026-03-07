## 2026-03-07 09:38: refactor: remove dead workspace code and tighten settings validation

**What changed:**
- Removed unused desktop UI primitives, legacy panel types, and the stale `pnpm-lock.yaml` file.
- Dropped unused Radix dependencies and trimmed unused exports from `use-panel-windows`.
- Promoted previously orphaned settings validation into the live settings store and deleted the unused settings domain module.
- Removed unused OAuth error variants and the unused Codex `refresh_token` field.
- Removed unused workspace crate memberships and deleted the code for `plugin-host`, `calendar-google`, and `event-bus`.

**Why:**
- Keep only code that is either live or intentionally integrated.
- Preserve the useful settings validation logic instead of deleting it as dead code.
- Reduce maintenance overhead from scaffolding and prototype crates that are no longer part of the active architecture.

**Files affected:**
- `Cargo.toml`
- `Cargo.lock`
- `apps/desktop-ui/package.json`
- `apps/desktop-ui/src/hooks/use-panel-windows.ts`
- `crates/peekoo-agent-app/src/settings/mod.rs`
- `crates/peekoo-agent-app/src/settings/store.rs`
- `crates/peekoo-agent-auth/src/error.rs`
- `crates/peekoo-agent-auth/src/provider/openai_codex.rs`
- `apps/desktop-ui/src/components/ui/card.tsx`
- `apps/desktop-ui/src/components/ui/separator.tsx`
- `apps/desktop-ui/src/components/ui/tooltip.tsx`
- `apps/desktop-ui/src/types/panel.ts`
- `apps/desktop-ui/pnpm-lock.yaml`
- `crates/peekoo-agent-app/src/settings/domain/mod.rs`
- `crates/peekoo-agent-app/src/settings/domain/agent_settings.rs`
- `crates/plugin-host/Cargo.toml`
- `crates/plugin-host/src/lib.rs`
- `crates/calendar-google/Cargo.toml`
- `crates/calendar-google/src/lib.rs`
- `crates/event-bus/Cargo.toml`
- `crates/event-bus/src/lib.rs`
