import { describe, expect, test } from "bun:test";
import { normalizeReleaseNotes } from "./release-notes";

describe("normalizeReleaseNotes", () => {
  test("returns null for missing notes", () => {
    expect(normalizeReleaseNotes(null)).toBeNull();
    expect(normalizeReleaseNotes(undefined)).toBeNull();
    expect(normalizeReleaseNotes("   ")).toBeNull();
  });

  test("normalizes line endings and trims whitespace", () => {
    const result = normalizeReleaseNotes("\r\n  ## Title\r\n- item\r\n  ");

    expect(result).toBe("## Title\n- item");
  });

  test("removes leading html comments from generated release notes", () => {
    const notes = "<!-- generated -->\n\n## What's Changed\n- one";

    expect(normalizeReleaseNotes(notes)).toBe("## What's Changed\n- one");
  });
});
