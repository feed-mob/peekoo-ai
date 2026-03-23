import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { TaskEvent } from "@/types/task";

export function useTaskActivity(taskId: string | null) {
  const [events, setEvents] = useState<TaskEvent[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const loadActivity = useCallback(async () => {
    if (!taskId) {
      setEvents([]);
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      const result = await invoke<TaskEvent[]>("get_task_activity", {
        taskId: taskId,
        limit: 50,
      });
      setEvents(result);
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)));
      console.error("Failed to load task activity:", err);
    } finally {
      setIsLoading(false);
    }
  }, [taskId]);

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
    loadActivity();

    // Refresh when window becomes visible
    const handleVisibilityChange = () => {
      if (document.visibilityState === "visible") {
        loadActivity();
      }
    };

    document.addEventListener("visibilitychange", handleVisibilityChange);

    return () => {
      document.removeEventListener("visibilitychange", handleVisibilityChange);
    };
  }, [loadActivity]);

  return {
    events,
    isLoading,
    error,
    reload: loadActivity,
    addComment,
    deleteEvent,
  };
}