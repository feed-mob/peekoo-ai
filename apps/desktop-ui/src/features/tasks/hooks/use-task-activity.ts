import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { TaskEvent } from "@/types/task";
import { TASKS_CHANGED_EVENT } from "../utils/task-activity";

const TASK_ACTIVITY_POLL_INTERVAL_MS = 5000;

export function useTaskActivity(taskId: string | null) {
  const [events, setEvents] = useState<TaskEvent[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [lastSyncedAt, setLastSyncedAt] = useState<number | null>(null);
  const [error, setError] = useState<Error | null>(null);

  const loadActivity = useCallback(async () => {
    if (!taskId) {
      setEvents([]);
      return;
    }

    setIsLoading(lastSyncedAt === null);
    setIsRefreshing(true);
    setError(null);

    try {
      const result = await invoke<TaskEvent[]>("get_task_activity", {
        taskId: taskId,
        limit: 50,
      });
      setEvents(result);
      setLastSyncedAt(Date.now());
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)));
      console.error("Failed to load task activity:", err);
    } finally {
      setIsRefreshing(false);
      setIsLoading(false);
    }
  }, [lastSyncedAt, taskId]);

  const addComment = useCallback(
    async (text: string) => {
      if (!taskId) return null;

      try {
        const event = await invoke<TaskEvent>("add_task_comment", {
          taskId: taskId,
          text: text,
          author: "user",
        });
        setEvents((prev) => [event, ...prev]);
        return event;
      } catch (err) {
        console.error("Failed to add comment:", err);
        throw err;
      }
    },
    [taskId]
  );

  const deleteEvent = useCallback(
    async (eventId: string) => {
      try {
        await invoke("delete_task_event", { eventId });
        setEvents((prev) => prev.filter((e) => e.id !== eventId));
      } catch (err) {
        console.error("Failed to delete event:", err);
        throw err;
      }
    },
    []
  );

  // Auto-load when taskId changes
  useEffect(() => {
    void loadActivity();

    const refresh = () => {
      void loadActivity();
    };

    const unlisten = listen<{ task_id?: string | null }>(TASKS_CHANGED_EVENT, (event) => {
      const changedTaskId = event.payload?.task_id;
      if (!taskId || !changedTaskId || changedTaskId === taskId) {
        refresh();
      }
    });

    // Refresh when window becomes visible
    const handleVisibilityChange = () => {
      if (document.visibilityState === "visible") {
        refresh();
      }
    };

    const intervalId = window.setInterval(() => {
      if (document.visibilityState === "visible") {
        refresh();
      }
    }, TASK_ACTIVITY_POLL_INTERVAL_MS);

    document.addEventListener("visibilitychange", handleVisibilityChange);

    return () => {
      window.clearInterval(intervalId);
      document.removeEventListener("visibilitychange", handleVisibilityChange);
      void unlisten.then((fn) => fn());
    };
  }, [loadActivity, taskId]);

  return {
    events,
    isLoading,
    isRefreshing,
    lastSyncedAt,
    error,
    reload: loadActivity,
    addComment,
    deleteEvent,
  };
}
