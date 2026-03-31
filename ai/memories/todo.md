# Peekoo AI - Implementation TODO

## Tauri Version (Current Focus)

### Completed

- [x] peekoo-node-runtime crate for Node.js management
  - Ported from Zed's node_runtime module (adapted for Tokio)
  - System Node.js detection (PATH lookup, >= v18.0.0)
  - Managed Node.js download (v20.18.0 LTS to `~/.peekoo/resources/node/`)
  - NPX package installation to per-agent directories
  - Archive extraction (tar.gz, zip)
  - See changelog: `ai/memories/changelogs/202603311430-feat-peekoo-node-runtime.md`

- [x] Agent Task Execution via ACP
  - Full ACP subprocess communication for agent task execution
  - AgentScheduler with 30-second polling for task execution
  - Agent registry table and task work tracking columns
  - Frontend agent selector for task assignment
  - See changelog: `ai/memories/changelogs/202603232317-feat-agent-task-execution.md`
  - PR #125: https://github.com/feed-mob/peekoo-ai/pull/125

- [x] ACP-native task MCP execution and follow-up mentions
  - Shared task MCP server passed through ACP `session/new` `mcpServers`
  - `peekoo-agent-acp` now runs the real `peekoo-agent` with bridged MCP task tools
  - Scheduler no longer writes fake task history; agent updates tasks through MCP tools
  - User `@peekoo-agent` comments re-queue agent-assigned tasks for follow-up work
  - Follow-up comments now trigger the scheduler immediately and recover stale `executing` tasks
  - Task follow-up context now uses comment-only history in chronological order with latest-comment emphasis
  - Per-task agent session reuse preserves context between follow-up runs and now falls back cleanly when no saved session exists yet
  - Desktop notifications fire for agent comments and agent status changes only
  - See changelog: `ai/memories/changelogs/202603240845-feat-agent-follow-up-mentions.md`
  - See changelog: `ai/memories/changelogs/202603240930-fix-agent-follow-up-trigger.md`
  - See changelog: `ai/memories/changelogs/202603240945-fix-task-session-fallback.md`

- [x] Project structure setup (agent-first workspace)
- [x] Core Rust business logic crates (`peekoo-agent`, `peekoo-agent-app`, `peekoo-agent-auth`, `peekoo-productivity-domain`, `persistence-sqlite`, `security`, `peekoo-paths`)
- [x] Tauri app scaffolding with 19 Rust commands (`greet`, `get_sprite_state`, `agent_prompt`, `agent_set_model`, `agent_get_model`, `agent_settings_get`, `agent_settings_update`, `agent_settings_catalog`, `agent_provider_auth_set_api_key`, `agent_provider_auth_clear`, `agent_provider_config_set`, `agent_oauth_start`, `agent_oauth_status`, `agent_oauth_cancel`, `create_task`, `pomodoro_start`, `pomodoro_pause`, `pomodoro_resume`, `pomodoro_finish`)
- [x] React UI components and styling
- [x] Pet UI with animations and moods
- [x] Tab-based navigation (Chat, Tasks, Pomodoro)
- [x] TypeScript and Vite configuration
- [x] Productivity domain tests passing
- [x] Agent service wrapping pi_agent_rust with persona file loading, skills, and auto-discovery
- [x] Chat panel with settings UI (provider/model selection, auth, skills)
- [x] Streaming responses via agent_prompt with Tauri event emission
- [x] Pomodoro backend commands (start/pause/resume/finish) wired to AgentApplication
- [x] Security crate with KeyringSecretStore, FileSecretStore, FallbackSecretStore
- [x] Settings input validation (non-empty provider/model, max_tool_iterations > 0)

### In Progress
- [ ] ACP Registry Integration - Support all ACP registry agents
  - [x] Research Zed's ACP implementation and registry format
  - [x] Create peekoo-node-runtime crate for Node.js/NPX support
  - [ ] Create acp-registry-client crate to fetch registry from CDN
  - [ ] Parse ACP registry JSON (agents with npx/binary/uvx distribution)
  - [ ] Add Tauri commands: `get_registry_agents()`, `install_registry_agent()`
  - [ ] Update "Available Runtimes" UI to fetch from registry instead of hardcoded
  - [ ] Support binary agent download (Cursor, Kimi CLI, etc.)
  - [ ] Platform-specific agent filtering (darwin/linux/windows, arch)
  - [ ] Cache registry with TTL (1 hour)
  - [ ] Support NPX agents: Gemini, Qwen Code, Cline, Auggie, etc.
  - [ ] Support binary agents: Cursor, Kimi CLI, Goose, etc.
  - [ ] Future: UVX agents (crow-cli, fast-agent)

- [ ] Evolve ACP/MCP into the primary agent runtime
  - [ ] Support OpenCode as another ACP provider
  - [ ] Ensure `peekoo-agent-acp` loads all available tools
  - [ ] Support loading a selected subset of tools from MCP
  - [ ] Convert remaining built-in tools into MCP tools
  - [ ] Make ACP the first-class agent runtime so ACP + MCP tools replace the current `peekoo-agent` path

- [x] Implement Tasks component with full CRUD
  - Connected to real SQLite-backed CRUD via Tauri commands
  - List with status filter tabs (All/Todo/In Progress/Done)
  - Status badge click-to-cycle on TaskItem
  - Hybrid labels (predefined + custom) with colored pills
  - User/agent assignment with icon display
  - Activity tab with grouped-by-day event feed
  - Agent tools (task_create/list/update/delete/toggle/assign) for LLM
  - Plugin host functions (peekoo_task_*) gated by "tasks" capability
  - Task activity summary injected into agent system prompt
  - See changelog: `ai/memories/changelogs/202603201200-feat-tasks-panel-full-crud.md`

- [ ] Implement Pomodoro timer UI
  - Actual countdown timer logic in frontend
  - Session tracking display
  - Notification triggers

- [x] Store conversation history in SQLite
  - Persist chat sessions across app restarts (pi handles JSONL; indexed via SQLite)
  - Load previous conversations on cold start (fixed CWD mismatch, PR #72)

### Planned (GPUI Version)
- [ ] Implement GPUI native UI as alternative
  - Native Rust window with pet animations
  - Event-driven architecture
  - Compare performance with Tauri version
  - Test on macOS/Linux only

### Polish
- [x] Add system tray icon
- [ ] Global keyboard shortcuts
- [ ] Sound effects for events
- [x] Desktop notifications
- [ ] Dark mode theme
- [x] Sprite window auto-resize for mini chat and reply bubble states
  - Main sprite window now auto-expands/shrinks for mini chat open/close
  - Expanded reading mode uses wider/taller constrained window sizing
  - Rust-managed resize constraints improve behavior on Linux/Wayland
  - See changelog: `ai/memories/changelogs/202603200347-fix-sprite-window-constrained-resize.md`

### Testing
- [ ] End-to-end integration tests
- [ ] Performance benchmarking
- [ ] Cross-platform testing (Windows/macOS/Linux for Tauri)
- [ ] Accessibility testing
- [ ] Security audit

### Technical Debt (from PR #140 review)
- [ ] Extract shared `next_mode_after_completion()` from duplicate completion paths
  - `peekoo-pomodoro-app/src/lib.rs`: `refresh_runtime_if_due` (~line 444) and `complete_due_session` (~line 696) both implement auto-advance + long-break-interval logic independently with subtle differences
  - Extract into a single pure function taking mode, completed_focus, and settings; returns (next_mode, next_minutes)
- [ ] Consolidate double polling (`usePomodoroWatcher` + `PomodoroPanel`)
  - Both poll `getPomodoroStatus` every 3s independently; when both mounted, two concurrent IPC calls per tick
  - Watcher's memo-trigger concern should be driven by backend events or a shared polling source
- [x] Refactor migration runner into table-driven loop
  - `settings/store.rs` `run_migrations_and_seed()` reduced from ~370 lines to ~70 lines
  - `build.rs` auto-discovers `migrations/*.sql` at compile time with metadata parsing
  - `MigrationDef` struct replaces 15 manual `pub const` declarations
  - SQL files renamed to timestamp prefixes with `-- @migrate:` metadata headers
  - Two helpers (`apply_create_migration`, `apply_alter_migration`) replace inline blocks
  - 6 new validation tests in `persistence-sqlite/src/lib.rs`
  - All 4 consumer files updated (`store.rs`, `app-settings`, `pomodoro-app`, `task-app`)
  - See plan: `ai/plans/migration-runner-refactor.md`
- [ ] Check affected row count in `save_pomodoro_memo` when `id = None`
  - `peekoo-pomodoro-app/src/lib.rs:244`: UPDATE via correlated subquery silently no-ops if no work cycles exist; caller gets `Ok(status)` with no indication memo wasn't saved
- [ ] Replace positional column indices in `load_status` with named access
  - `peekoo-pomodoro-app/src/lib.rs:578`: columns accessed by index (`row.get::<_, i64>(6)?`); if SELECT order changes, values silently read from wrong columns
  - Use `row.get::<_, T>("column_name")` instead
- [ ] Simplify `SpritePeekBadge` duplicate icon DOM into CSS drop-shadow
  - `SpritePeekBadge.tsx:187-246`: two overlapping DOM elements render the same icon for a glow effect; achievable with a single element + `filter: drop-shadow(...)`

---

**Last updated**: 2026-03-31

### Recent Major Changes (2026-03-31)
- [x] peekoo-node-runtime crate implementation
  - Full port of Zed's node_runtime to Tokio-based architecture
  - Foundation for ACP registry agent support (NPX and binary)
  - See changelog: `ai/memories/changelogs/202603311430-feat-peekoo-node-runtime.md`

### Recent Major Refactor (2026-03-21)
- [x] Complete Tasks UI refactoring
  - Optimistic updates with rollback on error
  - Drag-and-drop task reordering with persistence
  - Activity section in task detail view
  - Delete confirmation dialogs
  - Toast notifications for user feedback
  - Proper Date handling utilities
  - Fixed formatTimeRange bug (missing recurrence parameters)
  - See changelog: `ai/memories/changelogs/202603210345-feat-tasks-ui-refactor.md`
