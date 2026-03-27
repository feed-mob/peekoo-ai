## 2026-03-20 12:00: feat: Tasks panel — full CRUD, kanban list, labels, assignment, agent tools, plugin host functions, activity feed

**What changed:**
- Extended domain model with `assignee`, `labels`, `TaskEvent`, `TaskEventType`, `TaskService` trait
- Added DB migration `0005_task_extensions.sql` for assignee + labels columns
- Rewrote `ProductivityService` with full SQLite-backed CRUD + event writing on every mutation
- Added `task_activity_summary()` for agent context injection into system prompt
- Added 5 Tauri commands: `list_tasks`, `update_task`, `delete_task`, `toggle_task`, `task_list_events`
- Updated `create_task` to accept assignee + labels
- Created 6 agent tools: `task_create`, `task_list`, `task_update`, `task_delete`, `task_toggle`, `task_assign`
- Added `register_native_tools()` to `AgentService` for non-plugin tool registration
- Added 6 plugin host functions: `peekoo_task_create/list/update/delete/toggle/assign` (gated by `"tasks"` capability)
- Added `TaskService` trait to `peekoo-productivity-domain` so `peekoo-plugin-host` can use it without depending on `peekoo-agent-app`
- Updated `PluginRegistry::new()` to accept `Arc<dyn TaskService>`
- Rewrote frontend: TasksPanel with Tasks/Activity tab toggle, status filter tabs
- New components: TaskLabels, ActivityView, ActivityItem
- Updated TaskItem with status badge (click to cycle), assignee icon, label pills
- Updated TaskInput with assignee toggle + label picker
- New `use-tasks` hook with full CRUD via Tauri invoke
- Extended `Task` type with status/assignee/labels, added `TaskEvent` type
- Added 8 new persistence tests, all 185 workspace tests pass

**Why:**
- Issue #24: Connect Tasks panel to real task CRUD
- User requested kanban → list approach (340px panel too narrow for kanban columns)
- User requested hybrid labels (predefined + custom), user/agent assignment
- Agent needs task tools to manage tasks during conversation
- Plugins need host functions to operate on tasks
- Activity feed for both UI browsing and agent context injection

**Files affected:**
- `crates/peekoo-productivity-domain/src/task.rs` — Task extension, TaskEvent, TaskService trait, TaskDto
- `crates/peekoo-productivity-domain/Cargo.toml` — Added serde_json
- `crates/persistence-sqlite/migrations/0005_task_extensions.sql` — New migration
- `crates/persistence-sqlite/src/lib.rs` — Added MIGRATION_0005 constant
- `crates/peekoo-agent-app/src/productivity.rs` — Full CRUD rewrite with SQLite + events
- `crates/peekoo-agent-app/src/application.rs` — Pass db_conn, new delegation methods, agent context injection
- `crates/peekoo-agent-app/src/task_tools.rs` — New: 6 agent Tool implementations
- `crates/peekoo-agent-app/src/lib.rs` — Updated re-exports, added task_tools module
- `crates/peekoo-agent-app/src/settings/store.rs` — Added 0005 migration step
- `crates/peekoo-agent-app/Cargo.toml` — Added pi, async-trait dependencies
- `crates/peekoo-agent-app/tests/productivity_service.rs` — Rewrote with SQLite + new tests
- `crates/peekoo-agent/src/service.rs` — Added `register_native_tools()`
- `crates/peekoo-plugin-host/src/host_functions.rs` — 6 task host functions + HostContext.task_service
- `crates/peekoo-plugin-host/src/registry.rs` — Added task_service field + constructor param
- `crates/peekoo-plugin-host/Cargo.toml` — Added peekoo-productivity-domain dependency
- `crates/peekoo-plugin-store/Cargo.toml` — Added peekoo-productivity-domain for tests
- `crates/peekoo-plugin-store/src/lib.rs` — Updated test PluginRegistry::new() calls
- `apps/desktop-tauri/src-tauri/src/lib.rs` — New Tauri commands, handler registration
- `apps/desktop-ui/src/types/task.ts` — Extended Task type, added TaskEvent, PREDEFINED_LABELS
- `apps/desktop-ui/src/features/tasks/use-tasks.ts` — New: custom hook
- `apps/desktop-ui/src/features/tasks/TasksPanel.tsx` — Rewrite: tabs + filters
- `apps/desktop-ui/src/features/tasks/TaskItem.tsx` — Status badge, assignee, labels, drag handle
- `apps/desktop-ui/src/features/tasks/TaskInput.tsx` — Assignee toggle + label picker
- `apps/desktop-ui/src/features/tasks/TaskLabels.tsx` — New: colored pill badges
- `apps/desktop-ui/src/features/tasks/ActivityView.tsx` — New: grouped-by-day event feed
- `apps/desktop-ui/src/features/tasks/ActivityItem.tsx` — New: event row with icon + description
- `ai/memories/docs/tasks-panel.md` — Plan document
