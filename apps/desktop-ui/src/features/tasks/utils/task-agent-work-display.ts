import type { Task } from "@/types/task";
import type { TFunction } from "i18next";

function isAgentAssigned(task: Task): boolean {
  return task.assignee !== "user";
}

export function shouldShowAgentExecutingIndicator(task: Task): boolean {
  return isAgentAssigned(task) && task.agent_work_status === "executing";
}

export function getAgentFailureDetail(task: Task, t?: TFunction): string | null {
  if (!isAgentAssigned(task) || task.agent_work_status !== "failed") {
    return null;
  }

  const attempts = task.agent_work_attempt_count;
  if (!attempts) {
    return null;
  }

  if (t) {
    const key = attempts === 1 ? "tasks.agentWork.attempts_one" : "tasks.agentWork.attempts_other";
    return t(key, { count: attempts });
  }
  return `${attempts} ${attempts === 1 ? "attempt" : "attempts"}`;
}
