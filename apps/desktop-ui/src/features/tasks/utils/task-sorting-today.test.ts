import { describe, expect, test } from "bun:test";
import type { Task } from "@/types/task";
import { sortTasks } from "./task-sorting";

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

describe("sortTasks for today tab", () => {
  test("places done tasks after non-done tasks", () => {
    const tasks = [
      makeTask({ id: "done-today", status: "done", scheduled_start_at: "2026-03-25T09:00:00Z" }),
      makeTask({ id: "todo-today", status: "todo", scheduled_start_at: "2026-03-25T10:00:00Z" }),
      makeTask({ id: "progress-today", status: "in_progress", scheduled_start_at: "2026-03-25T08:00:00Z" }),
    ];

    const result = sortTasks(tasks, "today");

    expect(result.map((task) => task.id)).toEqual([
      "progress-today",
      "todo-today",
      "done-today",
    ]);
  });
});
