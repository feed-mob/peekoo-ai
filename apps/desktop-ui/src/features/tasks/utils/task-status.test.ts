import { describe, expect, test } from "bun:test";
import type { Task } from "@/types/task";
import { isTaskCompleted } from "./task-status";

function makeTask(overrides: Partial<Task> = {}): Task {
  return {
    id: "task-1",
    title: "Ship merge fix",
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

describe("isTaskCompleted", () => {
  test("returns true when the task status is done", () => {
    expect(isTaskCompleted(makeTask({ status: "done" }))).toBe(true);
  });

  test("returns false when the task status is not done", () => {
    expect(isTaskCompleted(makeTask({ status: "in_progress" }))).toBe(false);
  });
});
