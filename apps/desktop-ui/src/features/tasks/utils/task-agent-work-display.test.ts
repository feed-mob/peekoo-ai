import { describe, expect, test } from "bun:test";
import type { Task } from "@/types/task";
import {
  getAgentFailureDetail,
  shouldShowAgentExecutingIndicator,
} from "./task-agent-work-display";

function makeTask(overrides: Partial<Task> = {}): Task {
  return {
    id: "task-1",
    title: "Task",
    description: null,
    status: "todo",
    priority: "medium",
    assignee: "user",
    labels: [],
    scheduled_start_at: null,
    scheduled_end_at: null,
    estimated_duration_min: null,
    recurrence_rule: null,
    recurrence_time_of_day: null,
    parent_task_id: null,
    created_at: "2026-03-24T00:00:00Z",
    ...overrides,
  };
}

describe("shouldShowAgentExecutingIndicator", () => {
  test("shows indicator only for executing agent-assigned tasks", () => {
    expect(
      shouldShowAgentExecutingIndicator(
        makeTask({ assignee: "peekoo-agent", agent_work_status: "executing" })
      )
    ).toBe(true);

    expect(
      shouldShowAgentExecutingIndicator(
        makeTask({ assignee: "user", agent_work_status: "executing" })
      )
    ).toBe(false);
  });
});

describe("getAgentFailureDetail", () => {
  test("shows retry count only for failed agent tasks", () => {
    expect(
      getAgentFailureDetail(
        makeTask({
          assignee: "peekoo-agent",
          agent_work_status: "failed",
          agent_work_attempt_count: 2,
        })
      )
    ).toBe("2 attempts");
  });

  test("returns null for non-agent or non-failed tasks", () => {
    expect(
      getAgentFailureDetail(
        makeTask({ assignee: "user", agent_work_status: "failed", agent_work_attempt_count: 2 })
      )
    ).toBeNull();
    expect(
      getAgentFailureDetail(
        makeTask({ assignee: "peekoo-agent", agent_work_status: "executing", agent_work_attempt_count: 2 })
      )
    ).toBeNull();
  });
});
