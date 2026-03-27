import { describe, expect, test } from "bun:test";
import { STATUS_CONFIG, TASK_STATUS_OPTIONS } from "./task-formatting";

describe("TASK_STATUS_OPTIONS", () => {
  test("includes todo, in progress, and done statuses", () => {
    expect(TASK_STATUS_OPTIONS.map((option) => option.value)).toEqual([
      "todo",
      "in_progress",
      "done",
    ]);
  });

  test("reuses the labels from STATUS_CONFIG", () => {
    expect(TASK_STATUS_OPTIONS.map((option) => option.label)).toEqual([
      STATUS_CONFIG.todo.label,
      STATUS_CONFIG.in_progress.label,
      STATUS_CONFIG.done.label,
    ]);
  });
});
