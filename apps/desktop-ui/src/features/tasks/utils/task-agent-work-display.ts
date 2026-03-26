import type { Task } from "@/types/task";

function isAgentAssigned(task: Task): boolean {
  return task.assignee !== "user";
}

export function shouldShowAgentExecutingIndicator(task: Task): boolean {
  return isAgentAssigned(task) && task.agent_work_status === "executing";
}

export function getAgentFailureDetail(task: Task): string | null {
  if (!isAgentAssigned(task) || task.agent_work_status !== "failed") {
    return null;
  }

  const attempts = task.agent_work_attempt_count;
  if (!attempts) {
    return null;
  }

  return `${attempts} ${attempts === 1 ? "attempt" : "attempts"}`;
}
