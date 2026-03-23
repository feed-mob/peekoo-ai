## 2026-03-21 03:45: feat: Complete Tasks UI Refactoring with Optimistic Updates & Drag-and-Drop

**What changed:**
- Complete refactor of tasks UI architecture with new directory structure
- Added optimistic updates with rollback on error for all task operations
- Implemented drag-and-drop reordering using @dnd-kit with persistence
- Fixed critical bug: `formatTimeRange` was missing 2 required parameters (recurrenceRule, recurrenceTimeOfDay)
- Added proper date handling utilities (no more string slicing for dates)
- Added Activity section to TaskDetailView showing per-task activity history
- Added delete confirmation dialog for destructive actions
- Added toast notifications for user feedback on operations
- Simplified TaskQuickInput (removed scheduling - now only in detail view)
- Added loading states for individual operations (create, toggle, update, delete)
- Reorganized components into proper hierarchy: components/, hooks/, utils/

**Backend changes:**
- Migration 0008: Added order_index and created_at columns to tasks table
- New Tauri commands: `reorder_task` and `get_task_activity`
- Updated Task and TaskDto structs with order_index and created_at fields
- Fixed SQLite migration to handle duplicate column errors gracefully

**Files created:**
- Frontend utilities: `date-helpers.ts`, `task-formatting.ts`, `task-sorting.ts`
- Hooks: `use-toast.ts`, `use-task-operations.ts`, `use-task-activity.ts`
- Components: TaskList, SortableTaskItem, TaskListItem, TaskDetailView, TaskQuickInput, ActivityFeed, ActivityFeedItem, TaskActivitySection, TaskLabelPills, DeleteConfirmDialog, ErrorToast, LoadingSpinner
- Backend migration: `0008_task_order_index.sql`

**Files deleted:**
- Old implementation files: TaskItem.tsx, TaskDetailsView.tsx, TaskInput.tsx, ActivityView.tsx, ActivityItem.tsx, TaskLabels.tsx, use-tasks.ts (old version)

**Files modified:**
- `apps/desktop-ui/src/features/tasks/TasksPanel.tsx` - refactored main container
- `apps/desktop-ui/src/types/task.ts` - added order_index and created_at
- `apps/desktop-tauri/src-tauri/src/lib.rs` - added reorder_task and get_task_activity commands
- `crates/peekoo-agent-app/src/productivity.rs` - added reorder_task and get_task_activity methods
- `crates/peekoo-agent-app/src/application.rs` - added reorder_task and get_task_activity methods
- `crates/peekoo-agent-app/src/settings/store.rs` - added migration 0008 runner
- `crates/peekoo-productivity-domain/src/task.rs` - added order_index and created_at to Task and TaskDto
- `crates/peekoo-productivity-domain/src/task.rs` - added reorder_task and get_task_activity to TaskService trait
- `crates/peekoo-productivity-domain/Cargo.toml` - added chrono dependency
- `crates/persistence-sqlite/src/lib.rs` - added MIGRATION_0008_TASK_ORDER_INDEX export

**Key fixes:**
1. Fixed empty Select value bug using `__none__` sentinel value
2. Fixed TaskItem formatTimeRange call signature (was missing recurrence parameters)
3. Fixed parameter naming mismatch in get_task_activity (taskId vs task_id)
4. Fixed SQLite migration to handle duplicate column errors during development

**Architecture improvements:**
- Proper separation of concerns: utils, hooks, components
- Optimistic UI updates with automatic rollback on errors
- Consistent error handling with toast notifications
- Proper Date object usage instead of string manipulation
- Centralized formatting and sorting logic in utilities

**Dependencies added:**
- @dnd-kit/core, @dnd-kit/sortable, @dnd-kit/utilities for drag-and-drop
- chrono crate for Rust datetime handling

**Testing:**
- TypeScript compilation passes
- Rust compilation passes (cargo check)
- Unit tests pass (peekoo-productivity-domain)
- Test file updates needed: productivity_service.rs (Task::new signature changed)
