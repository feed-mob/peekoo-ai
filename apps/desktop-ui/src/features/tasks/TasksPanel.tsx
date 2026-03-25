import { useState } from "react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { CheckCircle2, Calendar, ListTodo, CheckCheck, CalendarDays, RefreshCw } from "lucide-react";
import type { TaskTab } from "@/types/task";
import { useTasks } from "./hooks/use-tasks";
import { TaskList } from "./components/TaskList";
import { TaskQuickInput } from "./components/TaskQuickInput";
import { ActivityFeed } from "./components/ActivityFeed";
import { TaskDetailView } from "./components/TaskDetailView";
import { ErrorToast } from "./components/ErrorToast";
import { LoadingSpinner } from "./components/LoadingSpinner";
import { formatSyncStatus } from "./utils/task-sync";
import { useTranslation } from "react-i18next";

const TAB_CONFIG: { value: TaskTab; labelKey: string; icon: React.ReactNode; emoji: string }[] = [
  { value: "today", labelKey: "tasks.tabs.today", icon: <CalendarDays size={13} />, emoji: "📅" },
  { value: "week", labelKey: "tasks.tabs.week", icon: <Calendar size={13} />, emoji: "📆" },
  { value: "all", labelKey: "tasks.tabs.all", icon: <ListTodo size={13} />, emoji: "📋" },
  { value: "done", labelKey: "tasks.tabs.done", icon: <CheckCheck size={13} />, emoji: "✅" },
];

function EmptyState({ tab }: { tab: TaskTab }) {
  const { t } = useTranslation();
  const messages: Record<TaskTab, { title: string; subtitle: string }> = {
    today: { title: t("tasks.empty.todayTitle"), subtitle: t("tasks.empty.todaySubtitle") },
    week: { title: t("tasks.empty.weekTitle"), subtitle: t("tasks.empty.weekSubtitle") },
    all: { title: t("tasks.empty.allTitle"), subtitle: t("tasks.empty.allSubtitle") },
    done: { title: t("tasks.empty.doneTitle"), subtitle: t("tasks.empty.doneSubtitle") },
  };
  const msg = messages[tab];

  return (
    <div className="flex flex-col items-center justify-center py-12 text-center">
      <CheckCircle2 size={48} className="text-text-muted/40 mb-3" />
      <p className="text-sm font-medium text-text-primary mb-1">{msg.title}</p>
      <p className="text-xs text-text-muted">{msg.subtitle}</p>
    </div>
  );
}

type MainTab = "tasks" | "activity";

export function TasksPanel() {
  const { t } = useTranslation();
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

  // Show loading state
  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-full">
        <LoadingSpinner />
        <span className="ml-2 text-sm text-text-muted">{t("tasks.loading")}</span>
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
          <h2 className="text-base font-semibold text-text-primary">{t("tasks.title")}</h2>
          <div className="mt-0.5 flex items-center gap-1.5 text-[10px] text-text-muted">
            <RefreshCw size={10} className={isRefreshing ? "animate-spin" : ""} />
            <span>{formatSyncStatus(isRefreshing, lastSyncedAt)}</span>
          </div>
        </div>
        <span className="text-xs text-text-muted font-medium">
          {t("tasks.doneCounter", { completed: stats.completed, total: stats.total })}
        </span>
      </div>

      {/* Main Tabs */}
      <div className="flex gap-1 bg-space-deep rounded-lg p-1">
        <button
          onClick={() => setMainTab("tasks")}
          className={`flex-1 py-1.5 text-xs font-medium rounded-md transition-colors ${
            mainTab === "tasks"
              ? "bg-space-surface text-text-primary shadow-sm"
              : "text-text-muted hover:text-text-primary"
          }`}
        >
          {t("tasks.mainTab.tasks")}
        </button>
        <button
          onClick={() => setMainTab("activity")}
          className={`flex-1 py-1.5 text-xs font-medium rounded-md transition-colors ${
            mainTab === "activity"
              ? "bg-space-surface text-text-primary shadow-sm"
              : "text-text-muted hover:text-text-primary"
          }`}
        >
          {t("tasks.mainTab.activity")}
        </button>
      </div>

      {mainTab === "tasks" ? (
        <>
          {/* Quick Input */}
          <TaskQuickInput onAdd={addTask} isCreating={isCreating} />

          {/* Time-based tabs */}
          <div className="flex gap-1">
            {TAB_CONFIG.map((tabConfig) => (
              <button
                key={tabConfig.value}
                onClick={() => setActiveTab(tabConfig.value)}
                className={`flex-1 flex items-center justify-center gap-1 px-2 py-1.5 text-[10px] font-medium rounded-md transition-colors ${
                  activeTab === tabConfig.value
                    ? "bg-[var(--glow-green)]/20 text-[var(--glow-green)] border border-[var(--glow-green)]/40"
                    : "bg-space-deep text-text-muted border border-glass-border hover:text-text-primary"
                }`}
              >
                {tabConfig.icon}
                <span className="hidden sm:inline">{t(tabConfig.labelKey)}</span>
                <span className="sm:hidden">{tabConfig.emoji}</span>
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
      <ErrorToast toasts={toasts} onRemove={removeToast} />
    </div>
  );
}
