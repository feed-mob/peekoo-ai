import { useState, useEffect, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Task, TaskEvent, TaskStatus, TaskTab } from "@/types/task";
import { emitPetReaction } from "@/lib/pet-events";
import { useToast } from "./use-toast";
import { filterTasksByTab, sortTasks } from "../utils/task-sorting";
import { TASKS_CHANGED_EVENT } from "../utils/task-activity";
import { getCheckboxToggleStatus } from "../utils/task-interactions";

const TASKS_POLL_INTERVAL_MS = 5000;

export function useTasks() {
  const { toasts, removeToast, success, error } = useToast();

  // Core state
  const [tasks, setTasks] = useState<Task[]>([]);
  const [activityEvents, setActivityEvents] = useState<TaskEvent[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [lastSyncedAt, setLastSyncedAt] = useState<number | null>(null);
  const [globalError, setGlobalError] = useState<Error | null>(null);

  // Navigation state
  const [activeTab, setActiveTab] = useState<TaskTab>("today");
  const [selectedTaskId, setSelectedTaskId] = useState<string | null>(null);

  // Operation loading states
  const [isCreating, setIsCreating] = useState(false);
  const [isToggling, setIsToggling] = useState<string | null>(null);
  const [isUpdating, setIsUpdating] = useState<string | null>(null);
  const [isDeleting, setIsDeleting] = useState<string | null>(null);

  // Load tasks from backend
  const loadTasks = useCallback(async () => {
    try {
      setIsRefreshing(true);
      setGlobalError(null);
      const result = await invoke<Task[]>("list_tasks");
      setTasks(result);
      setLastSyncedAt(Date.now());
    } catch (err) {
      const errObj = err instanceof Error ? err : new Error(String(err));
      setGlobalError(errObj);
      error("Failed to load tasks");
      console.error("Failed to load tasks:", err);
    } finally {
      setIsRefreshing(false);
      setIsLoading(false);
    }
  }, [error]);

  // Load activity events
  const loadEvents = useCallback(
    async (limit = 50) => {
      try {
        const result = await invoke<TaskEvent[]>("task_list_events", { limit });
        setActivityEvents(result);
      } catch (err) {
        console.error("Failed to load task events:", err);
      }
    },
    []
  );

  // Create task from natural language text (AI-powered parsing)
  const addTask = useCallback(
    async (text: string) => {
      setIsCreating(true);
      try {
        const task = await invoke<Task>("create_task_from_text", { text });
        setTasks((prev) => [task, ...prev]);
        success("Task created");
        return task;
      } catch (err) {
        // Fallback: create with whole text as title
        try {
          const task = await invoke<Task>("create_task", {
            title: text,
            priority: "medium",
            assignee: "user",
            labels: [],
            description: null,
            scheduled_start_at: null,
            scheduled_end_at: null,
            estimated_duration_min: null,
            recurrence_rule: null,
            recurrence_time_of_day: null,
          });
          setTasks((prev) => [task, ...prev]);
          success("Task created");
          return task;
        } catch (fallbackErr) {
          error("Failed to create task");
          console.error("Failed to create task:", fallbackErr);
          throw fallbackErr;
        }
      } finally {
        setIsCreating(false);
      }
    },
    [success, error]
  );

  // Toggle task completion (optimistic)
  const toggleTask = useCallback(
    async (id: string) => {
      const currentTask = tasks.find((t) => t.id === id);
      if (!currentTask) return;

      const optimisticStatus: TaskStatus = getCheckboxToggleStatus(currentTask.status);
      const shouldCelebrate = optimisticStatus === "done";

      // Optimistic update
      setTasks((prev) =>
        prev.map((t) =>
          t.id === id ? { ...t, status: optimisticStatus } : t
        )
      );
      setIsToggling(id);

      try {
        const result = await invoke<Task>("toggle_task", { id });
        setTasks((prev) => prev.map((t) => (t.id === id ? result : t)));

        if (shouldCelebrate && result.status === "done") {
          void emitPetReaction("task-completed");
          success("Task completed!");
        }

        // Reload to get any new recurring task instances and stay aligned with background changes.
        void loadTasks();
      } catch (err) {
        // Rollback
        setTasks((prev) =>
          prev.map((t) => (t.id === id ? currentTask : t))
        );
        error("Failed to update task");
        console.error("Failed to toggle task:", err);
      } finally {
        setIsToggling(null);
      }
    },
    [tasks, success, error, loadTasks]
  );

  // Update task fields (optimistic)
  const updateTask = useCallback(
    async (id: string, fields: Partial<Task>) => {
      const currentTask = tasks.find((t) => t.id === id);
      if (!currentTask) return;

      // Optimistic update
      setTasks((prev) =>
        prev.map((t) => (t.id === id ? { ...t, ...fields } : t))
      );
      setIsUpdating(id);

      try {
        // Transform snake_case fields to camelCase for Tauri backend
        const payload: Record<string, unknown> = { id };
        for (const [key, value] of Object.entries(fields)) {
          if (key === "recurrence_rule") {
            payload.recurrenceRule = value;
          } else if (key === "recurrence_time_of_day") {
            payload.recurrenceTimeOfDay = value;
          } else if (key === "scheduled_start_at") {
            payload.scheduled_start_at = value;
          } else if (key === "scheduled_end_at") {
            payload.scheduled_end_at = value;
          } else if (key === "estimated_duration_min") {
            payload.estimated_duration_min = value;
          } else {
            payload[key] = value;
          }
        }
        const result = await invoke<Task>("update_task", payload);
        setTasks((prev) => prev.map((t) => (t.id === id ? result : t)));
      } catch (err) {
        // Rollback
        setTasks((prev) =>
          prev.map((t) => (t.id === id ? currentTask : t))
        );
        error("Failed to save changes");
        console.error("Failed to update task:", err);
      } finally {
        setIsUpdating(null);
      }
    },
    [tasks, error]
  );

  // Update just status
  const updateTaskStatus = useCallback(
    async (id: string, status: TaskStatus) => {
      await updateTask(id, { status });
    },
    [updateTask]
  );

  // Delete task (optimistic)
  const deleteTask = useCallback(
    async (id: string) => {
      const currentTask = tasks.find((t) => t.id === id);
      if (!currentTask) return;

      // Optimistic update
      setTasks((prev) => prev.filter((t) => t.id !== id));
      if (selectedTaskId === id) {
        setSelectedTaskId(null);
      }
      setIsDeleting(id);

      try {
        await invoke("delete_task", { id });
        success("Task deleted");
      } catch (err) {
        // Rollback
        setTasks((prev) => [...prev, currentTask]);
        error("Failed to delete task");
        console.error("Failed to delete task:", err);
      } finally {
        setIsDeleting(null);
      }
    },
    [tasks, selectedTaskId, success, error]
  );

  // Computed values
  const filteredTasks = useMemo(() => {
    const today = new Date();
    const weekEnd = new Date(today.getTime() + 7 * 24 * 60 * 60 * 1000);
    return filterTasksByTab(tasks, activeTab, today, weekEnd);
  }, [tasks, activeTab]);

  const sortedTasks = useMemo(() => {
    return sortTasks(filteredTasks, activeTab);
  }, [activeTab, filteredTasks]);

  const selectedTask = useMemo(
    () => tasks.find((t) => t.id === selectedTaskId) || null,
    [tasks, selectedTaskId]
  );

  const stats = useMemo(() => {
    const total = tasks.length;
    const completed = tasks.filter((t) => t.status === "done").length;
    return { total, completed };
  }, [tasks]);

  // Initial load + background refresh
  useEffect(() => {
    void loadTasks();
    void loadEvents();

    const refresh = () => {
      void loadTasks();
      void loadEvents();
    };

    const unlisten = listen(TASKS_CHANGED_EVENT, refresh);

    const handleVisibilityChange = () => {
      if (document.visibilityState === "visible") {
        refresh();
      }
    };

    const intervalId = window.setInterval(() => {
      if (document.visibilityState === "visible") {
        refresh();
      }
    }, TASKS_POLL_INTERVAL_MS);

    document.addEventListener("visibilitychange", handleVisibilityChange);

    return () => {
      window.clearInterval(intervalId);
      document.removeEventListener("visibilitychange", handleVisibilityChange);
      void unlisten.then((fn) => fn());
    };
  }, [loadEvents, loadTasks]);

  return {
    // Data
    tasks: sortedTasks,
    allTasks: tasks,
    activityEvents,
    stats,

    // Loading states
    isLoading,
    isRefreshing,
    lastSyncedAt,
    isCreating,
    isToggling,
    isUpdating,
    isDeleting,

    // Errors
    globalError,
    toasts,
    removeToast,

    // Navigation
    activeTab,
    setActiveTab,
    selectedTaskId,
    setSelectedTaskId,
    selectedTask,

    // Actions
    addTask,
    toggleTask,
    updateTask,
    updateTaskStatus,
    deleteTask,
    reload: loadTasks,
    reloadEvents: loadEvents,
  };
}
