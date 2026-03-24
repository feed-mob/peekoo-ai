export interface Task {
  id: string;
  title: string;
  description: string | null;
  status: "todo" | "in_progress" | "done";
  priority: "low" | "medium" | "high";
  assignee: string;
  labels: string[];
  scheduled_start_at: string | null;
  scheduled_end_at: string | null;
  estimated_duration_min: number | null;
  recurrence_rule: string | null;
  recurrence_time_of_day: string | null;
  parent_task_id: string | null;
  created_at: string;
  agent_work_status?: "pending" | "claimed" | "executing" | "completed" | "failed";
  agent_work_attempt_count?: number;
}

export interface TaskEvent {
  id: string;
  task_id: string;
  event_type: string;
  payload: Record<string, unknown>;
  created_at: string;
}

export interface Agent {
  id: string;
  name: string;
  capabilities: string[];
}

export const KNOWN_AGENTS: Agent[] = [
  { id: "user", name: "Me", capabilities: [] },
  { id: "peekoo-agent", name: "Peekoo Agent", capabilities: ["task_planning", "task_execution", "question_asking"] },
];

export const PREDEFINED_LABELS = [
  { name: "bug", color: "#E5484D" },
  { name: "feature", color: "#30A46C" },
  { name: "urgent", color: "#E9762B" },
  { name: "design", color: "#7B61FF" },
  { name: "docs", color: "#7B9AC7" },
  { name: "refactor", color: "#F5C842" },
  // Agent-specific labels
  { name: "agent_working", color: "#3B82F6" },
  { name: "needs_clarification", color: "#F59E0B" },
  { name: "agent_done", color: "#10B981" },
  { name: "needs_review", color: "#8B5CF6" },
  { name: "agent_failed", color: "#EF4444" },
] as const;

export type TaskStatus = Task["status"];
export type TaskPriority = Task["priority"];

export type TaskTab = "today" | "week" | "all" | "done";
