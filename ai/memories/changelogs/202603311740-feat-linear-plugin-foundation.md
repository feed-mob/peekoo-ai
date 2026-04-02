## 2026-03-31 17:40: feat: add Linear plugin foundation with task sync and settings status

**What changed:**
- Added a new first-party plugin at `plugins/linear` with OAuth connect/disconnect, panel snapshot provider, manual sync trigger, periodic scheduler sync, and two-way task sync scaffolding against Linear GraphQL.
- Added a new `tasks` module in `crates/peekoo-plugin-sdk` and wired raw host function bindings so plugins can create/list/update/delete/toggle/assign Peekoo tasks safely.
- Added a new settings hook and UI section to show Linear integration status in Settings, including install/enable/connected/sync/error surfaces.
- Updated `justfile` to include `linear` in `plugin-build-all`.

**Why:**
- Implement the approved design direction for Linear as an install-gated independent plugin.
- Enable plugin-side background synchronization without coupling Linear logic into core app crates.
- Satisfy acceptance criteria that connection status is visible in Settings.

**Files affected:**
- `plugins/linear/Cargo.toml`
- `plugins/linear/peekoo-plugin.toml`
- `plugins/linear/src/lib.rs`
- `plugins/linear/ui/panel.html`
- `plugins/linear/ui/panel.css`
- `plugins/linear/ui/panel.js`
- `crates/peekoo-plugin-sdk/src/host_fns.rs`
- `crates/peekoo-plugin-sdk/src/tasks.rs`
- `crates/peekoo-plugin-sdk/src/lib.rs`
- `apps/desktop-ui/src/features/settings/useLinearIntegrationStatus.ts`
- `apps/desktop-ui/src/features/settings/SettingsPanel.tsx`
- `justfile`
