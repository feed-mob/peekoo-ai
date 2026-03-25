import { describe, expect, test } from "bun:test";
import { getDoneTaskVisualStyle } from "./task-visuals";

describe("getDoneTaskVisualStyle", () => {
  test("uses stronger deemphasis for done tasks in today tab", () => {
    expect(getDoneTaskVisualStyle(true, true)).toBe("opacity-45 saturate-75");
  });

  test("uses default deemphasis for done tasks outside today tab", () => {
    expect(getDoneTaskVisualStyle(true, false)).toBe("opacity-60");
  });

  test("does not change visuals for non-done tasks", () => {
    expect(getDoneTaskVisualStyle(false, true)).toBe("");
  });
});
