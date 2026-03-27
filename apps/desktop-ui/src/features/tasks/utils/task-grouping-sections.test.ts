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

describe("splitTodayTasks sections", () => {
  test("splits today tasks into overdue, today, unscheduled active, and completed sections", () => {
    const today = new Date("2026-03-25T12:00:00Z");
    const tasks = [
      makeTask({ id: "overdue", scheduled_start_at: "2026-03-24T09:00:00Z" }),
      makeTask({ id: "today", scheduled_start_at: "2026-03-25T14:00:00Z" }),
      makeTask({ id: "unscheduled" }),
      makeTask({
        id: "done",
        status: "done",
        scheduled_start_at: "2026-03-25T08:00:00Z",
        finished_at: "2026-03-25T10:00:00Z",
      }),
      makeTask({
        id: "done-yesterday",
        status: "done",
        scheduled_start_at: null,
        finished_at: "2026-03-24T10:00:00Z",
      }),
    ];

    const result = splitTodayTasks(tasks, today);

    expect(result.overdue.map((task) => task.id)).toEqual(["overdue"]);
    expect(result.today.map((task) => task.id)).toEqual(["today"]);
    expect(result.unscheduled.map((task) => task.id)).toEqual(["unscheduled"]);
    expect(result.completed.map((task) => task.id)).toEqual(["done"]);
  });
});
