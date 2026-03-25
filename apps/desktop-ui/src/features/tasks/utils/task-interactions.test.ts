import { describe, expect, test } from "bun:test";
import { getCheckboxToggleStatus } from "./task-interactions";

describe("getCheckboxToggleStatus", () => {
  test("marks todo tasks as done when checked", () => {
    expect(getCheckboxToggleStatus("todo")).toBe("done");
  });

  test("marks in progress tasks as done when checked", () => {
    expect(getCheckboxToggleStatus("in_progress")).toBe("done");
  });

  test("restores done tasks to in progress when unchecked", () => {
    expect(getCheckboxToggleStatus("done")).toBe("in_progress");
  });
});
