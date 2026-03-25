import type { TaskStatus } from "@/types/task";

export function getCheckboxToggleStatus(status: TaskStatus): TaskStatus {
  return status === "done" ? "in_progress" : "done";
}
