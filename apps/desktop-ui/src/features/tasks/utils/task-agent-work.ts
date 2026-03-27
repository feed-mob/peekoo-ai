import type { Task } from "@/types/task";

export interface AgentWorkStatusBadge {
  label: string;
  color: string;
  animated: boolean;
}

const AGENT_WORK_STATUS_BADGES: Partial<
  Record<NonNullable<Task["agent_work_status"]>, AgentWorkStatusBadge>
> = {
  pending: { label: "Pending", color: "#F59E0B", animated: false },
  executing: { label: "Executing", color: "#3B82F6", animated: true },
  failed: { label: "Failed", color: "#EF4444", animated: false },
};

export function getAgentWorkStatusBadge(
  status: Task["agent_work_status"]
): AgentWorkStatusBadge | null {
  return status ? AGENT_WORK_STATUS_BADGES[status] ?? null : null;
}
