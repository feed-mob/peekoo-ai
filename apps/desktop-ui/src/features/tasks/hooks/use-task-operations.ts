import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Task, TaskStatus } from "@/types/task";
import { emitPetReaction } from "@/lib/pet-events";

interface UseTaskOperationsProps {
  tasks: Task[];
  setTasks: React.Dispatch<React.SetStateAction<Task[]>>;
  addToast: (params: { type: "success" | "error"; message: string }) => void;
}

export function useTaskOperations({ tasks, setTasks, addToast }: UseTaskOperationsProps) {
  // Loading states per operation
  const [isToggling, setIsToggling] = useState<string | null>(null);
  const [isUpdating, setIsUpdating] = useState<string | null>(null);
  const [isDeleting, setIsDeleting] = useState<string | null>(null);

  /**
   * Toggle task completion with optimistic update
   */
  const toggleTask = useCallback(
    async (id: string) => {
      const currentTask = tasks.find((t) => t.id === id);
      if (!currentTask) return;

      const shouldCelebrate = currentTask.status !== "done";
      const optimisticStatus: TaskStatus =
        currentTask.status === "done" ? "todo" : "done";

      // Optimistic update
      setTasks((prev) =>
        prev.map((t) =>
          t.id === id ? { ...t, status: optimisticStatus } : t
        )
      );
      setIsToggling(id);

      try {
        const result = await invoke<Task>("toggle_task", { id });
        // Confirm with server result
        setTasks((prev) => prev.map((t) => (t.id === id ? result : t)));

        if (shouldCelebrate && result.status === "done") {
          void emitPetReaction("task-completed");
          addToast({ type: "success", message: "Task completed!" });
        }
      } catch (err) {
        // Rollback on error
        setTasks((prev) =>
          prev.map((t) => (t.id === id ? currentTask : t))
        );
        addToast({ type: "error", message: "Failed to update task" });
        console.error("Failed to toggle task:", err);
      } finally {
        setIsToggling(null);
      }
    },
    [tasks, setTasks, addToast]
  );

  /**
   * Update task fields with optimistic update
   */
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
        const result = await invoke<Task>("update_task", { id, ...fields });
        // Confirm with server result
        setTasks((prev) => prev.map((t) => (t.id === id ? result : t)));
      } catch (err) {
        // Rollback on error
        setTasks((prev) =>
          prev.map((t) => (t.id === id ? currentTask : t))
        );
        addToast({ type: "error", message: "Failed to save changes" });
        console.error("Failed to update task:", err);
      } finally {
        setIsUpdating(null);
      }
    },
    [tasks, setTasks, addToast]
  );

  /**
   * Update just the status (for badge cycling)
   */
  const updateTaskStatus = useCallback(
    async (id: string, status: TaskStatus) => {
      await updateTask(id, { status });
    },
    [updateTask]
  );

  /**
   * Delete task with optimistic update
   */
  const deleteTask = useCallback(
    async (id: string) => {
      const currentTask = tasks.find((t) => t.id === id);
      if (!currentTask) return;

      // Optimistic update
      setTasks((prev) => prev.filter((t) => t.id !== id));
      setIsDeleting(id);

      try {
        await invoke("delete_task", { id });
        addToast({ type: "success", message: "Task deleted" });
      } catch (err) {
        // Rollback on error
        setTasks((prev) => [...prev, currentTask]);
        addToast({ type: "error", message: "Failed to delete task" });
        console.error("Failed to delete task:", err);
      } finally {
        setIsDeleting(null);
      }
    },
    [tasks, setTasks, addToast]
  );

  return {
    // Operations
    toggleTask,
    updateTask,
    updateTaskStatus,
    deleteTask,
    // Loading states
    isToggling,
    isUpdating,
    isDeleting,
  };
}
