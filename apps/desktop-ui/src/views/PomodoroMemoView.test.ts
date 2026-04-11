import { describe, expect, mock, test } from "bun:test";
import { prepareMemoTaskChoices, submitPomodoroMemo } from "./PomodoroMemoView";
import type { Task } from "@/types/task";

function makeTask(overrides: Partial<Task>): Task {
  return {
    id: overrides.id ?? "task-id",
    title: overrides.title ?? "Task",
    description: null,
    status: overrides.status ?? "todo",
    priority: "medium",
    assignee: "user",
    labels: [],
    scheduled_start_at: overrides.scheduled_start_at ?? null,
    scheduled_end_at: null,
    estimated_duration_min: null,
    recurrence_rule: null,
    recurrence_time_of_day: null,
    parent_task_id: null,
    created_at: overrides.created_at ?? "2026-04-11T10:00:00Z",
    updated_at: overrides.updated_at,
    finished_at: null,
  };
}

describe("submitPomodoroMemo", () => {
  test("saves the memo, comments on the selected task, and closes the window", async () => {
    const saveMemo = mock(async () => {});
    const closeWindow = mock(async () => {});

    await submitPomodoroMemo({
      memo: "Deep work notes",
      taskId: "task-123",
      saveMemo,
      closeWindow,
    });

    expect(saveMemo).toHaveBeenCalledWith(null, "Deep work notes", "task-123");
    expect(closeWindow).toHaveBeenCalledTimes(1);
  });

  test("skips task comments when no task is selected", async () => {
    const saveMemo = mock(async () => {});
    const closeWindow = mock(async () => {});

    await submitPomodoroMemo({
      memo: "Deep work notes",
      taskId: null,
      saveMemo,
      closeWindow,
    });

    expect(saveMemo).toHaveBeenCalledWith(null, "Deep work notes", null);
    expect(closeWindow).toHaveBeenCalledTimes(1);
  });

  test("saves and closes when memo is empty", async () => {
    const saveMemo = mock(async () => {});
    const closeWindow = mock(async () => {});

    await submitPomodoroMemo({
      memo: "   ",
      taskId: "task-123",
      saveMemo,
      closeWindow,
    });

    expect(saveMemo).toHaveBeenCalledWith(null, "   ", "task-123");
    expect(closeWindow).toHaveBeenCalledTimes(1);
  });
});

describe("prepareMemoTaskChoices", () => {
  test("sorts tasks by in-progress first and nearest schedule", () => {
    const tasks = [
      makeTask({
        id: "todo-near",
        title: "Todo Near",
        status: "todo",
        updated_at: "2026-04-11T09:00:00Z",
        scheduled_start_at: "2026-04-11T11:50:00Z",
      }),
      makeTask({
        id: "in-progress-far",
        title: "In Progress Far",
        status: "in_progress",
        updated_at: "2026-04-11T12:00:00Z",
        scheduled_start_at: "2026-04-11T20:00:00Z",
      }),
      makeTask({
        id: "in-progress-near",
        title: "In Progress Near",
        status: "in_progress",
        updated_at: "2026-04-11T08:00:00Z",
        scheduled_start_at: "2026-04-11T12:10:00Z",
      }),
      makeTask({ id: "done-task", title: "Done", status: "done" }),
    ];

    const result = prepareMemoTaskChoices(tasks, Date.parse("2026-04-11T12:00:00Z"));

    expect(result.tasks.map((task) => task.id)).toEqual([
      "in-progress-near",
      "in-progress-far",
      "todo-near",
    ]);
    expect(result.defaultTaskId).toBe("in-progress-near");
  });

  test("falls back to newest activity when schedules are missing", () => {
    const tasks = [
      makeTask({
        id: "todo-new",
        title: "Todo New",
        status: "todo",
        updated_at: "2026-04-11T12:00:00Z",
      }),
      makeTask({
        id: "todo-old",
        title: "Todo Old",
        status: "todo",
        updated_at: "2026-04-11T08:00:00Z",
      }),
    ];

    const result = prepareMemoTaskChoices(tasks, Date.parse("2026-04-11T12:00:00Z"));

    expect(result.tasks.map((task) => task.id)).toEqual(["todo-new", "todo-old"]);
    expect(result.defaultTaskId).toBe("todo-new");
  });

  test("returns no default when no active tasks exist", () => {
    const tasks = [makeTask({ id: "done-task", status: "done" })];

    const result = prepareMemoTaskChoices(tasks);

    expect(result.tasks).toEqual([]);
    expect(result.defaultTaskId).toBeNull();
  });
});
