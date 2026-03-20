import { useMemo, useState } from "react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { CheckCircle2 } from "lucide-react";
import type { TaskStatus } from "@/types/task";
import { useTasks } from "./use-tasks";
import { TaskItem } from "./TaskItem";
import { TaskInput } from "./TaskInput";
import { ActivityView } from "./ActivityView";

type Tab = "tasks" | "activity";
type StatusFilter = "all" | TaskStatus;

const STATUS_FILTERS: { value: StatusFilter; label: string }[] = [
  { value: "all", label: "All" },
  { value: "todo", label: "Todo" },
  { value: "in_progress", label: "In Progress" },
  { value: "done", label: "Done" },
];

function EmptyState() {
  return (
    <div className="flex flex-col items-center justify-center py-12 text-center">
      <CheckCircle2 size={48} className="text-text-muted/40 mb-3" />
      <p className="text-sm font-medium text-text-primary mb-1">No tasks yet</p>
      <p className="text-xs text-text-muted">Add your first task to get started</p>
    </div>
  );
}

export function TasksPanel() {
  const {
    tasks,
    activityEvents,
    loading,
    addTask,
    toggleTask,
    updateTaskStatus,
    deleteTask,
  } = useTasks();

  const [tab, setTab] = useState<Tab>("tasks");
  const [statusFilter, setStatusFilter] = useState<StatusFilter>("all");

  const filteredTasks = useMemo(() => {
    if (statusFilter === "all") return tasks;
    return tasks.filter((t) => t.status === statusFilter);
  }, [tasks, statusFilter]);

  const stats = useMemo(() => {
    const total = tasks.length;
    const completed = tasks.filter((t) => t.status === "done").length;
    return { total, completed };
  }, [tasks]);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <p className="text-sm text-text-muted">Loading tasks...</p>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full gap-3">
      {/* Header with stats */}
      <div className="flex items-center justify-between">
        <h2 className="text-base font-semibold text-text-primary">Tasks</h2>
        <span className="text-xs text-text-muted font-medium">
          {stats.completed} / {stats.total} completed
        </span>
      </div>

      {/* Tabs */}
      <div className="flex gap-1 bg-space-deep rounded-lg p-1">
        <button
          onClick={() => setTab("tasks")}
          className={`flex-1 py-1.5 text-xs font-medium rounded-md transition-colors ${
            tab === "tasks"
              ? "bg-space-surface text-text-primary shadow-sm"
              : "text-text-muted hover:text-text-primary"
          }`}
        >
          Tasks
        </button>
        <button
          onClick={() => setTab("activity")}
          className={`flex-1 py-1.5 text-xs font-medium rounded-md transition-colors ${
            tab === "activity"
              ? "bg-space-surface text-text-primary shadow-sm"
              : "text-text-muted hover:text-text-primary"
          }`}
        >
          Activity
        </button>
      </div>

      {tab === "tasks" ? (
        <>
          {/* Input */}
          <TaskInput onAdd={addTask} />

          {/* Status filters */}
          <div className="flex gap-1">
            {STATUS_FILTERS.map((f) => (
              <button
                key={f.value}
                onClick={() => setStatusFilter(f.value)}
                className={`px-2.5 py-1 text-[10px] font-medium rounded-full transition-colors ${
                  statusFilter === f.value
                    ? "bg-[var(--glow-green)]/20 text-[var(--glow-green)] border border-[var(--glow-green)]/40"
                    : "bg-space-deep text-text-muted border border-glass-border hover:text-text-primary"
                }`}
              >
                {f.label}
              </button>
            ))}
          </div>

          {/* Task list */}
          <ScrollArea className="flex-1 -mx-1 px-1">
            {filteredTasks.length === 0 ? (
              <EmptyState />
            ) : (
              <div className="space-y-2 pr-2">
                {filteredTasks.map((task) => (
                  <TaskItem
                    key={task.id}
                    task={task}
                    onToggle={() => toggleTask(task.id)}
                    onDelete={() => deleteTask(task.id)}
                    onStatusChange={(status) => updateTaskStatus(task.id, status)}
                  />
                ))}
              </div>
            )}
          </ScrollArea>
        </>
      ) : (
        <ScrollArea className="flex-1 -mx-1 px-1">
          <div className="pr-2">
            <ActivityView events={activityEvents} />
          </div>
        </ScrollArea>
      )}
    </div>
  );
}
