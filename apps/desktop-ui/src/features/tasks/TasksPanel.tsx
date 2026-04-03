import { useState } from "react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { CalendarDays, Calendar, ListTodo, CheckCheck, CheckCircle2, RefreshCw } from "lucide-react";
import type { TaskTab } from "@/types/task";
import { useTasks } from "./hooks/use-tasks";
import { TaskList } from "./components/TaskList";
import { TaskQuickInput } from "./components/TaskQuickInput";
import { ActivityFeed } from "./components/ActivityFeed";
import { TaskDetailView } from "./components/TaskDetailView";
import { NotificationToast } from "./components/ErrorToast";
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

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-full">
        <LoadingSpinner />
        <span className="ml-2 text-sm text-text-muted">{t("tasks.loading")}</span>
      </div>
    );
  }

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

      <div className="flex gap-1 bg-glass backdrop-blur-xl rounded-lg p-1 border border-glass-border/50 mb-3">
        <button
          onClick={() => setMainTab("tasks")}
          className={`flex-1 py-2 text-xs font-medium rounded-md transition-all duration-200 ${
            mainTab === "tasks"
              ? "bg-glow-green dark:bg-glow-olive text-white dark:text-space-void shadow-md"
              : "text-text-muted hover:text-text-primary hover:bg-space-overlay/30"
          }`}
        >
          {t("tasks.mainTab.tasks")}
        </button>
        <button
          onClick={() => setMainTab("activity")}
          className={`flex-1 py-2 text-xs font-medium rounded-md transition-all duration-200 ${
            mainTab === "activity"
              ? "bg-glow-green dark:bg-glow-olive text-white dark:text-space-void shadow-md"
              : "text-text-muted hover:text-text-primary hover:bg-space-overlay/30"
          }`}
        >
          {t("tasks.mainTab.activity")}
        </button>
      </div>

      {mainTab === "tasks" ? (
        <>
          <TaskQuickInput onAdd={addTask} isCreating={isCreating} />

          <div className="flex gap-1 mb-4">
            {TAB_CONFIG.map((tabConfig) => (
              <button
                key={tabConfig.value}
                onClick={() => setActiveTab(tabConfig.value)}
                className={`relative flex-1 flex items-center justify-center gap-1 px-2 py-2 text-[11px] font-medium rounded-md transition-all duration-200 ${
                  activeTab === tabConfig.value
                    ? "bg-glow-green/15 dark:bg-glow-olive/20 text-glow-green dark:text-glow-olive"
                    : "bg-space-deep text-text-muted hover:text-text-primary hover:bg-space-overlay/30"
                }`}
              >
                {tabConfig.icon}
                <span className="hidden sm:inline">{t(tabConfig.labelKey)}</span>
                <span className="sm:hidden">{tabConfig.emoji}</span>
                {activeTab === tabConfig.value && (
                  <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-glow-green dark:bg-glow-olive rounded-full" />
                )}
              </button>
            ))}
          </div>

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

      <NotificationToast toasts={toasts} onRemove={removeToast} />
    </div>
  );
}
