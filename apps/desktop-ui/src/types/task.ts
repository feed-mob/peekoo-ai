export interface Task {
  id: string;
  title: string;
  status: "todo" | "in_progress" | "done";
  priority: "low" | "medium" | "high";
  assignee: "user" | "agent";
  labels: string[];
}

export interface TaskEvent {
  id: string;
  task_id: string;
  event_type: string;
  payload: Record<string, unknown>;
  created_at: string;
}

export const PREDEFINED_LABELS = [
  { name: "bug", color: "#E5484D" },
  { name: "feature", color: "#30A46C" },
  { name: "urgent", color: "#E9762B" },
  { name: "design", color: "#7B61FF" },
  { name: "docs", color: "#7B9AC7" },
  { name: "refactor", color: "#F5C842" },
] as const;

export type TaskStatus = Task["status"];
export type TaskPriority = Task["priority"];
