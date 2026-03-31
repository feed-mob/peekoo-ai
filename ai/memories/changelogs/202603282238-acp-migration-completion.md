## 2026-03-28 22:38: feat: Complete ACP migration - single DB, real provider management, session restore

**What changed:**
- Consolidated agent session persistence to use single `peekoo.sqlite` database instead of separate `agent_sessions.db`
- Reimplemented conversation session restore from SQLite-backed `agent_sessions` and `session_messages` tables
- Refactored `AgentProviderService` to be Tauri-safe with `Arc<Mutex<Connection>>` pattern
- Replaced all mock ACP provider Tauri commands with real implementations calling `AgentApplication`
- Added provider management methods to `AgentApplication` (list, install, uninstall, set default, config, test, prerequisites, custom providers)
- Updated provider catalog to use ACP providers (pi-acp, opencode, claude-code, codex) instead of legacy providers
- Mounted `AgentProviderPanel` in Settings view with full provider management UI
- Added `QuickProviderSwitcher` to chat panel header for mid-conversation provider switching
- Updated chat settings panel to hide auth section for ACP providers (they manage their own auth)
- Added missing Radix UI dependencies (@radix-ui/react-dialog, dropdown-menu, label, radio-group)
- Fixed all TypeScript build errors in provider UI components
- Added tests for session restore and message flattening

**Why:**
- The ACP (Agent Client Protocol) migration was incomplete with stubbed/mock implementations
- Session persistence was fragmented across multiple database files
- Provider management UI existed but wasn't wired to functional backend
- This completes the migration to use ACP providers as the primary architecture

**Files affected:**
- `crates/peekoo-agent/src/service.rs` - Use shared peekoo.sqlite path
- `crates/peekoo-agent-app/src/conversation.rs` - Reimplement session restore
- `crates/peekoo-agent-app/src/agent_provider_service.rs` - Thread-safe refactoring
- `crates/peekoo-agent-app/src/application.rs` - Add provider management methods
- `crates/peekoo-agent-app/src/settings/catalog.rs` - ACP provider catalog
- `apps/desktop-tauri/src-tauri/src/lib.rs` - Real provider commands
- `apps/desktop-ui/src/features/settings/SettingsPanel.tsx` - Mount provider UI
- `apps/desktop-ui/src/features/chat/ChatPanel.tsx` - Quick provider switcher
- `apps/desktop-ui/src/features/chat/settings/ChatSettingsPanel.tsx` - Hide auth for ACP
- `apps/desktop-ui/package.json` - Add Radix UI deps
- `ai/plans/2026-03-28-acp-migration-completion.md` - Execution plan

**Testing:**
- All 257 tests passing (43 suites)
- Provider catalog tests verify ACP providers listed
- Session restore tests verify SQLite-backed conversation loading
- TypeScript build passes
