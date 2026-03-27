import type { Task } from "@/types/task";

export function isTaskCompleted(task: Task): boolean {
  return task.status === "done";
}
