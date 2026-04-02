import { useState } from "react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { CheckCircle2, Link2, RefreshCw } from "lucide-react";
import type { TaskTab } from "@/types/task";
import { useTasks } from "./hooks/use-tasks";
import { TaskList } from "./components/TaskList";
import { TaskQuickInput } from "./components/TaskQuickInput";
import { ActivityFeed } from "./components/ActivityFeed";
import { TaskDetailView } from "./components/TaskDetailView";
import { NotificationToast } from "./components/ErrorToast";
import { LoadingSpinner } from "./components/LoadingSpinner";
import { formatSyncStatus } from "./utils/task-sync";
import { usePlugins } from "@/hooks/use-plugins";
import { openPanelWindow } from "@/hooks/use-panel-windows";
import type { PanelLabel } from "@/types/window";

const TAB_CONFIG: { value: TaskTab; label: string; emoji: string }[] = [
  { value: "today", label: "Today", emoji: "📅" },
  { value: "week", label: "This Week", emoji: "📆" },
  { value: "all", label: "All", emoji: "📋" },
  { value: "done", label: "Done", emoji: "✅" },
];

function EmptyState({ tab }: { tab: TaskTab }) {
  const messages: Record<TaskTab, { title: string; subtitle: string }> = {
    today: { title: "No tasks for today", subtitle: "Schedule a task or add a new one" },
    week: { title: "Nothing this week", subtitle: "Schedule tasks for the upcoming week" },
    all: { title: "No tasks yet", subtitle: "Create your first task to get started" },
    done: { title: "No completed tasks", subtitle: "Complete some tasks to see them here" },
  };
  const msg = messages[tab];

  return (
    <div className="flex flex-col items-center justify-center py-16 text-center">
      <div className="relative mb-4">
        <div className="absolute inset-0 bg-gradient-to-br from-glow-green to-glow-olive dark:from-glow-olive dark:to-glow-mint opacity-20 blur-2xl rounded-full" />
        <CheckCircle2 
          size={56} 
          className="relative text-glow-green dark:text-glow-olive animate-pulse" 
          strokeWidth={1.5}
        />
      </div>
      <p className="text-sm font-medium text-text-secondary mb-1">{msg.title}</p>
      <p className="text-xs text-text-muted">{msg.subtitle}</p>
    </div>
  );
}

type MainTab = "tasks" | "activity";

export function TasksPanel() {
  const { plugins, panels: pluginPanels } = usePlugins();
  const {
    tasks,
    activityEvents,
    stats,
    isLoading,
    isRefreshing,
    lastSyncedAt,
    isCreating,
    isToggling,
    isUpdating,
    isDeleting,
    toasts,
    removeToast,
    activeTab,
    setActiveTab,
    setSelectedTaskId,
    selectedTask,
    addTask,
    toggleTask,
    updateTask,
    updateTaskStatus,
    deleteTask,
  } = useTasks();

  const [mainTab, setMainTab] = useState<MainTab>("tasks");
  const linearPlugin = plugins.find((plugin) => plugin.pluginKey === "linear");
  const linearPanel = pluginPanels.find((panel) => panel.pluginKey === "linear");
  const showLinearButton = Boolean(linearPlugin && linearPanel);
  const canOpenLinearPanel = Boolean(linearPanel);

  // Show loading state
  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-full">
        <LoadingSpinner />
        <span className="ml-2 text-sm text-text-muted">Loading tasks...</span>
      </div>
    );
  }

  // Detail view
  if (selectedTask) {
    return (
      <TaskDetailView
        task={selectedTask}
        onBack={() => setSelectedTaskId(null)}
        onUpdate={(fields) => updateTask(selectedTask.id, fields)}
        onToggle={() => toggleTask(selectedTask.id)}
        onDelete={() => deleteTask(selectedTask.id)}
        isUpdating={isUpdating === selectedTask.id}
        isDeleting={isDeleting === selectedTask.id}
      />
    );
  }

  return (
    <div className="flex flex-col h-full gap-3">
      {/* Header with stats */}
      <div className="flex items-center justify-between gap-3">
        <div>
          <h2 className="text-base font-semibold text-text-primary">Tasks</h2>
          <div className="mt-0.5 flex items-center gap-1.5 text-[10px] text-text-muted">
            <RefreshCw size={10} className={isRefreshing ? "animate-spin" : ""} />
            <span>{formatSyncStatus(isRefreshing, lastSyncedAt)}</span>
          </div>
        </div>
        <span className="text-xs text-text-muted font-medium">
          {stats.completed} / {stats.total} done
        </span>
      </div>

      {showLinearButton ? (
        <div className="flex items-center justify-between gap-2 rounded-lg border border-glass-border/50 bg-glass/60 px-2 py-1.5">
          <span className="text-[10px] uppercase tracking-wider text-text-muted">Sources</span>
          <button
            type="button"
            onClick={() => {
              if (!linearPanel) {
                return;
              }
              void openPanelWindow(linearPanel.label as PanelLabel, pluginPanels);
            }}
            disabled={!canOpenLinearPanel}
            className="h-8 px-3 rounded-full shrink-0 flex items-center gap-1.5 text-xs font-medium text-text-primary bg-glass border border-glass-border hover:bg-space-overlay/40 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            title={canOpenLinearPanel ? "Open Linear panel" : "Enable Linear plugin to open panel"}
          >
            <Link2 size={13} />
            <span>Linear</span>
          </button>
        </div>
      ) : null}

      {/* Main Tabs */}
      <div className="flex gap-1 bg-glass backdrop-blur-xl rounded-lg p-1 border border-glass-border/50 mb-3">
        <button
          onClick={() => setMainTab("tasks")}
          className={`flex-1 py-2 text-xs font-medium rounded-md transition-all duration-200 ${
            mainTab === "tasks"
              ? "bg-glow-green dark:bg-glow-olive text-white dark:text-space-void shadow-md"
              : "text-text-muted hover:text-text-primary hover:bg-space-overlay/30"
          }`}
        >
          Tasks
        </button>
        <button
          onClick={() => setMainTab("activity")}
          className={`flex-1 py-2 text-xs font-medium rounded-md transition-all duration-200 ${
            mainTab === "activity"
              ? "bg-glow-green dark:bg-glow-olive text-white dark:text-space-void shadow-md"
              : "text-text-muted hover:text-text-primary hover:bg-space-overlay/30"
          }`}
        >
          Activity
        </button>
      </div>

      {mainTab === "tasks" ? (
        <>
          {/* Quick Input */}
          <TaskQuickInput onAdd={addTask} isCreating={isCreating} />

          {/* Time-based tabs */}
          <div className="flex gap-1 mb-4">
            {TAB_CONFIG.map((t) => (
              <button
                key={t.value}
                onClick={() => setActiveTab(t.value)}
                className={`relative flex-1 flex items-center justify-center gap-1 px-2 py-2 text-[11px] font-medium rounded-md transition-all duration-200 ${
                  activeTab === t.value
                    ? "bg-glow-green/15 dark:bg-glow-olive/20 text-glow-green dark:text-glow-olive"
                    : "bg-space-deep text-text-muted hover:text-text-primary hover:bg-space-overlay/30"
                }`}
              >
                <span>{t.label}</span>
                {activeTab === t.value && (
                  <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-glow-green dark:bg-glow-olive rounded-full" />
                )}
              </button>
            ))}
          </div>

          {/* Task list */}
          <ScrollArea className="flex-1 -mx-1 px-1">
            {tasks.length === 0 ? (
              <EmptyState tab={activeTab} />
            ) : (
              <TaskList
                tasks={tasks}
                onToggle={toggleTask}
                onDelete={deleteTask}
                onStatusChange={updateTaskStatus}
                onSelect={setSelectedTaskId}
                isTodayTab={activeTab === "today"}
                isToggling={isToggling}
                isUpdating={isUpdating}
                isDeleting={isDeleting}
              />
            )}
          </ScrollArea>
        </>
      ) : (
        <ScrollArea className="flex-1 -mx-1 px-1">
          <div className="pr-2">
            <ActivityFeed events={activityEvents} />
          </div>
        </ScrollArea>
      )}

      {/* Toast notifications */}
      <NotificationToast toasts={toasts} onRemove={removeToast} />
    </div>
  );
}
