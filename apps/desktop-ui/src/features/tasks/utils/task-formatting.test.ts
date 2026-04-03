import { describe, expect, test } from "bun:test";
import { STATUS_CONFIG, getTaskStatusOptions } from "./task-formatting";

const mockT = ((key: string) => key) as import("i18next").TFunction;

describe("getTaskStatusOptions", () => {
  test("includes todo, in progress, and done statuses", () => {
    expect(getTaskStatusOptions(mockT).map((option) => option.value)).toEqual([
      "todo",
      "in_progress",
      "done",
    ]);
  });

  test("reuses the labelKeys from STATUS_CONFIG", () => {
    expect(getTaskStatusOptions(mockT).map((option) => option.label)).toEqual([
      STATUS_CONFIG.todo.labelKey,
      STATUS_CONFIG.in_progress.labelKey,
      STATUS_CONFIG.done.labelKey,
    ]);
  });
});
