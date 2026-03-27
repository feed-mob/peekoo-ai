import { describe, expect, test } from "bun:test";
import type { Task } from "@/types/task";
import { splitTodayTasks } from "./task-grouping";

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
    updated_at: "2026-03-24T00:00:00Z",
    finished_at: null,
    ...overrides,
  };
}

describe("splitTodayTasks unscheduled done", () => {
  test("places unscheduled done tasks in completed section only when finished today", () => {
    const result = splitTodayTasks(
      [
        makeTask({
          id: "done-unscheduled",
          status: "done",
          scheduled_start_at: null,
          finished_at: "2026-03-25T08:00:00Z",
        }),
        makeTask({
          id: "done-earlier",
          status: "done",
          scheduled_start_at: null,
          finished_at: "2026-03-24T08:00:00Z",
        }),
      ],
      new Date("2026-03-25T12:00:00Z")
    );

    expect(result.completed.map((task) => task.id)).toEqual(["done-unscheduled"]);
  });
});
