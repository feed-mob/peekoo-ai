import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Task, TaskEvent, TaskStatus } from "@/types/task";
import { emitPetReaction } from "@/lib/pet-events";

export function useTasks() {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [activityEvents, setActivityEvents] = useState<TaskEvent[]>([]);
  const [loading, setLoading] = useState(true);

  const loadTasks = useCallback(async () => {
    try {
      const result = await invoke<Task[]>("list_tasks");
      setTasks(result);
    } catch (err) {
      console.error("Failed to load tasks:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  const loadEvents = useCallback(async (limit = 50) => {
    try {
      const result = await invoke<TaskEvent[]>("task_list_events", { limit });
      setActivityEvents(result);
    } catch (err) {
      console.error("Failed to load task events:", err);
    }
  }, []);

  const addTask = useCallback(
    async (title: string, priority: Task["priority"], assignee: Task["assignee"] = "user", labels: string[] = []) => {
      try {
        const task = await invoke<Task>("create_task", { title, priority, assignee, labels });
        setTasks((prev) => [task, ...prev]);
      } catch (err) {
        console.error("Failed to create task:", err);
      }
    },
    [],
  );

  const toggleTask = useCallback(async (id: string) => {
    const current = tasks.find((t) => t.id === id);
    const shouldCelebrate = current && current.status !== "done";

    try {
      const updated = await invoke<Task>("toggle_task", { id });
      setTasks((prev) => prev.map((t) => (t.id === id ? updated : t)));
      if (shouldCelebrate) {
        void emitPetReaction("task-completed");
      }
    } catch (err) {
      console.error("Failed to toggle task:", err);
    }
  }, [tasks]);

  const updateTaskStatus = useCallback(async (id: string, status: TaskStatus) => {
    try {
      const updated = await invoke<Task>("update_task", { id, status });
      setTasks((prev) => prev.map((t) => (t.id === id ? updated : t)));
    } catch (err) {
      console.error("Failed to update task status:", err);
    }
  }, []);

  const updateTask = useCallback(
    async (id: string, fields: Partial<Pick<Task, "title" | "priority" | "status" | "assignee" | "labels">>) => {
      try {
        const updated = await invoke<Task>("update_task", { id, ...fields });
        setTasks((prev) => prev.map((t) => (t.id === id ? updated : t)));
      } catch (err) {
        console.error("Failed to update task:", err);
      }
    },
    [],
  );

  const deleteTask = useCallback(async (id: string) => {
    try {
      await invoke("delete_task", { id });
      setTasks((prev) => prev.filter((t) => t.id !== id));
    } catch (err) {
      console.error("Failed to delete task:", err);
    }
  }, []);

  useEffect(() => {
    loadTasks();
    loadEvents();
  }, [loadTasks, loadEvents]);

  return {
    tasks,
    activityEvents,
    loading,
    addTask,
    toggleTask,
    updateTaskStatus,
    updateTask,
    deleteTask,
    reload: loadTasks,
    reloadEvents: loadEvents,
  };
}
