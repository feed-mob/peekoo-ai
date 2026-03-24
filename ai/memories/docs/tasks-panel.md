# Tasks Panel — Full Feature Plan

## Context

Issue #24: Connect Tasks panel to real task CRUD. The current `TasksPanel.tsx` uses hardcoded mock data. The `tasks` table exists in SQLite but is unused. Only `create_task` Tauri command exists (in-memory, no persistence).

**Scope**: List with status filters, hybrid labels (predefined + custom), user/agent assignment, SQLite-backed CRUD, agent tools (LLM can call task operations), plugin host functions (WASM plugins can call task operations), activity feed (task events for UI + agent context).

## Architecture

Three surfaces access task operations, all delegating to the same `ProductivityService`:

- **React UI** → Tauri commands → ProductivityService → SQLite
- **Agent (LLM)** → pi::Tool impl → ProductivityService → SQLite
- **Plugin (WASM)** → host functions → ProductivityService → SQLite

## Implementation Steps

### Step 1: Domain Model
- `crates/peekoo-productivity-domain/src/task.rs`: Extend `Task` with `assignee: String`, `labels: Vec<String>`. Add `TaskEventType`, `TaskEvent`, `TaskService` trait.

### Step 2: DB Migration
- `crates/persistence-sqlite/migrations/0005_task_extensions.sql`: ALTER TABLE tasks ADD assignee, labels_json columns. Keep existing `task_events` table.

### Step 3: App Layer CRUD + Events
- `crates/peekoo-agent-app/src/productivity.rs`: Inject `Arc<Mutex<Connection>>`. Full CRUD + event writing on every mutation. Add `list_task_events()`, `task_activity_summary()`.

### Step 4: Tauri Commands
- `apps/desktop-tauri/src-tauri/src/lib.rs`: Add `list_tasks`, `update_task`, `delete_task`, `toggle_task`, `task_list_events`. Update `create_task`.

### Step 5: Agent Tools
- `crates/peekoo-agent-app/src/task_tools.rs`: Implement `pi::tools::Tool` for `task_create`, `task_list`, `task_update`, `task_delete`, `task_toggle`, `task_assign`.
- `crates/peekoo-agent/src/service.rs`: Add `register_native_tools()`.

### Step 6: Plugin Host Functions
- `crates/peekoo-plugin-host/src/host_functions.rs`: Add `peekoo_task_create`, `peekoo_task_list`, `peekoo_task_update`, `peekoo_task_delete`, `peekoo_task_toggle`, `peekoo_task_assign`. Require capability `"tasks"`.
- `crates/peekoo-plugin-host/src/registry.rs`: Accept `Arc<dyn TaskService>`.

### Step 7: Frontend Types
- `apps/desktop-ui/src/types/task.ts`: Extend `Task` with status, assignee, labels. Add `TaskEvent`. Add `PREDEFINED_LABELS`.

### Step 8: Custom Hook
- `apps/desktop-ui/src/features/tasks/use-tasks.ts`: All invoke calls, loading state, activity events.

### Step 9: List UI
- `TasksPanel.tsx`: Rewrite with Tasks/Activity tab toggle + status filters.
- `TaskItem.tsx`: Status badge, label pills, assignee icon.
- `TaskInput.tsx`: Assignee toggle + label picker.
- `TaskLabels.tsx`: Colored pill badges.

### Step 10: Activity Tab
- `ActivityView.tsx`: Groups events by day, renders ActivityItem rows.
- `ActivityItem.tsx`: Icon + description + relative time.

### Step 11: Tests
- `crates/peekoo-agent-app/tests/productivity_service.rs`: Persistence + event tests.

## Agent Context Injection
- In `AgentApplication::resolved_config()`, inject `task_activity_summary()` into agent system prompt.

## Acceptance Criteria
- [ ] Tasks panel loads real tasks from SQLite
- [ ] CRUD operations persist + write events
- [ ] Status filter tabs: All / Todo / In Progress / Done
- [ ] Labels display as colored pills, predefined + custom
- [ ] User/agent assignee on cards
- [ ] Agent tools callable by LLM
- [ ] Plugin host functions callable by WASM
- [ ] Activity tab with grouped events
- [ ] Agent system prompt includes task summary
- [ ] Pet celebration on completion
- [ ] All Rust tests pass, frontend type-checks clean
