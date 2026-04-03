import type { Task } from "@/types/task";
import type { TFunction } from "i18next";

export interface AgentWorkStatusBadge {
  label: string;
  color: string;
  animated: boolean;
}

const AGENT_WORK_STATUS_BADGES: Partial<
  Record<NonNullable<Task["agent_work_status"]>, Omit<AgentWorkStatusBadge, "label"> & { labelKey: string }>
> = {
  pending: { labelKey: "tasks.agentWork.pending", color: "#F59E0B", animated: false },
  executing: { labelKey: "tasks.agentWork.executing", color: "#3B82F6", animated: true },
  failed: { labelKey: "tasks.agentWork.failed", color: "#EF4444", animated: false },
};

export function getAgentWorkStatusBadge(
  status: Task["agent_work_status"],
  t: TFunction
): AgentWorkStatusBadge | null {
  if (!status) return null;
  const config = AGENT_WORK_STATUS_BADGES[status];
  if (!config) return null;
  return {
    label: t(config.labelKey),
    color: config.color,
    animated: config.animated,
  };
}
