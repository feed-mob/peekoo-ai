import { describe, expect, test } from "bun:test";
import { shouldForwardConsole } from "./bootstrap";

describe("shouldForwardConsole", () => {
  test("disables Tauri log forwarding during dev", () => {
    expect(shouldForwardConsole(true)).toBe(false);
  });

  test("keeps Tauri log forwarding outside dev", () => {
    expect(shouldForwardConsole(false)).toBe(true);
  });
});
