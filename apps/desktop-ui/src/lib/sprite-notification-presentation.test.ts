import { describe, expect, test } from "bun:test";

import {
  DEFAULT_SPRITE_BUBBLE_DURATION_MS,
  PRIORITY_SPRITE_BUBBLE_DURATION_MS,
  getSpriteBubbleDurationMs,
  getSpriteBubbleKind,
} from "./sprite-notification-presentation";

describe("getSpriteBubbleDurationMs", () => {
  test("keeps default duration for generic notifications", () => {
    expect(
      getSpriteBubbleDurationMs({
        title: "Reminder",
        body: "Take a break",
      }),
    ).toBe(DEFAULT_SPRITE_BUBBLE_DURATION_MS);
  });

  test("extends duration for task reminders", () => {
    expect(
      getSpriteBubbleDurationMs({
        sourcePlugin: "tasks",
        title: "Task reminder",
        body: "Join standup starts now",
      }),
    ).toBe(PRIORITY_SPRITE_BUBBLE_DURATION_MS);
  });

  test("extends duration for calendar reminders", () => {
    expect(
      getSpriteBubbleDurationMs({
        sourcePlugin: "google-calendar",
        title: "Design review",
        body: "Starts at 10:30",
      }),
    ).toBe(PRIORITY_SPRITE_BUBBLE_DURATION_MS);
  });
});

describe("getSpriteBubbleKind", () => {
  test("classifies task reminders for stronger styling", () => {
    expect(
      getSpriteBubbleKind({
        sourcePlugin: "tasks",
        title: "Task reminder",
        body: "Join standup starts now",
      }),
    ).toBe("task");
  });

  test("classifies calendar reminders for stronger styling", () => {
    expect(
      getSpriteBubbleKind({
        sourcePlugin: "google-calendar",
        title: "Design review",
        body: "Starts at 10:30",
      }),
    ).toBe("calendar");
  });
});
