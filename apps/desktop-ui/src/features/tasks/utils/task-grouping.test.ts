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

describe("splitTodayTasks", () => {
  test("places unscheduled active tasks separately and only includes tasks finished today in completed", () => {
    const tasks = [
      makeTask({ id: "todo-1", status: "todo" }),
      makeTask({ id: "done-1", status: "done", finished_at: "2026-03-25T09:00:00Z" }),
      makeTask({ id: "progress-1", status: "in_progress" }),
    ];

    const result = splitTodayTasks(tasks, new Date("2026-03-25T12:00:00Z"));

    expect(result.overdue).toEqual([]);
    expect(result.today).toEqual([]);
    expect(result.unscheduled.map((task) => task.id)).toEqual(["todo-1", "progress-1"]);
    expect(result.completed.map((task) => task.id)).toEqual(["done-1"]);
  });
});
