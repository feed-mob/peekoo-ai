import { describe, expect, test } from "bun:test";
import type { Task } from "@/types/task";
import { filterTasksByTab, sortTasks } from "./task-sorting";

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

describe("filterTasksByTab", () => {
  test("today includes overdue, today, and unscheduled active tasks, plus tasks finished today", () => {
    const tasks = [
      makeTask({
        id: "done-today",
        status: "done",
        scheduled_start_at: "2026-03-25T09:00:00Z",
        finished_at: "2026-03-25T11:00:00Z",
      }),
      makeTask({
        id: "todo-today",
        status: "todo",
        scheduled_start_at: "2026-03-25T10:00:00Z",
      }),
      makeTask({
        id: "overdue",
        status: "todo",
        scheduled_start_at: "2026-03-24T09:00:00Z",
      }),
      makeTask({
        id: "unscheduled",
        status: "in_progress",
        scheduled_start_at: null,
      }),
      makeTask({
        id: "unscheduled-done",
        status: "done",
        scheduled_start_at: null,
        finished_at: "2026-03-25T07:30:00Z",
      }),
      makeTask({
        id: "done-before-today",
        status: "done",
        scheduled_start_at: null,
        finished_at: "2026-03-24T07:30:00Z",
      }),
      makeTask({
        id: "tomorrow",
        status: "todo",
        scheduled_start_at: "2026-03-26T09:00:00Z",
      }),
    ];

    const result = filterTasksByTab(
      tasks,
      "today",
      new Date("2026-03-25T12:00:00Z"),
      new Date("2026-04-01T12:00:00Z")
    );

    expect(result.map((task) => task.id)).toEqual([
      "done-today",
      "todo-today",
      "overdue",
      "unscheduled",
      "unscheduled-done",
    ]);
  });
});

describe("sortTasks", () => {
  test("sorts done tasks by finished_at descending with fallbacks", () => {
    const tasks = [
      makeTask({
        id: "created-fallback",
        status: "done",
        created_at: "2026-03-20T08:00:00Z",
        updated_at: "2026-03-20T08:00:00Z",
        finished_at: null,
      }),
      makeTask({
        id: "updated-fallback",
        status: "done",
        created_at: "2026-03-19T08:00:00Z",
        updated_at: "2026-03-24T09:00:00Z",
        finished_at: null,
      }),
      makeTask({
        id: "finished-newer",
        status: "done",
        created_at: "2026-03-10T08:00:00Z",
        updated_at: "2026-03-24T08:00:00Z",
        finished_at: "2026-03-25T10:00:00Z",
      }),
      makeTask({
        id: "finished-older",
        status: "done",
        created_at: "2026-03-24T08:00:00Z",
        updated_at: "2026-03-25T08:00:00Z",
        finished_at: "2026-03-25T09:00:00Z",
      }),
    ];

    const result = sortTasks(tasks, "done");

    expect(result.map((task) => task.id)).toEqual([
      "finished-newer",
      "finished-older",
      "updated-fallback",
      "created-fallback",
    ]);
  });
});
