import { describe, expect, test } from "bun:test";
import type { TaskEvent } from "@/types/task";
import { getCommentAuthorDisplayName } from "./task-activity";

function makeCommentEvent(payload: Record<string, unknown>): TaskEvent {
  return {
    id: "event-1",
    task_id: "task-1",
    event_type: "comment",
    payload,
    created_at: "2026-03-25T10:00:00Z",
  };
}

describe("getCommentAuthorDisplayName", () => {
  test("shows known agent names for agent comments", () => {
    const event = makeCommentEvent({ author: "peekoo-agent", text: "Done" });

    expect(getCommentAuthorDisplayName(event)).toBe("Peekoo Agent");
  });

  test("falls back to raw author name for unknown agents", () => {
    const event = makeCommentEvent({ author: "builder-agent", text: "Done" });

    expect(getCommentAuthorDisplayName(event)).toBe("builder-agent");
  });

  test("shows You for user comments", () => {
    const event = makeCommentEvent({ author: "user", text: "Please retry" });

    expect(getCommentAuthorDisplayName(event)).toBe("You");
  });
});
