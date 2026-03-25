import { describe, expect, test } from "bun:test";
import { deriveCountdownSnapshot } from "./countdown.ts";

describe("deriveCountdownSnapshot", () => {
  test("decrements running timers based on elapsed seconds since last sync", () => {
    const result = deriveCountdownSnapshot(
      {
        mode: "work",
        state: "Running",
        minutes: 25,
        time_remaining_secs: 10,
        completed_focus: 0,
        completed_breaks: 0,
        enable_memo: false,
        default_work_minutes: 25,
        default_break_minutes: 5,
      },
      1_000,
      4_200,
    );

    expect(result.timeRemainingSecs).toBe(7);
    expect(result.progress).toBe(99.53333333333333);
  });

  test("does not decrement paused timers", () => {
    const result = deriveCountdownSnapshot(
      {
        mode: "work",
        state: "Paused",
        minutes: 25,
        time_remaining_secs: 10,
        completed_focus: 0,
        completed_breaks: 0,
        enable_memo: false,
        default_work_minutes: 25,
        default_break_minutes: 5,
      },
      1_000,
      9_500,
    );

    expect(result.timeRemainingSecs).toBe(10);
  });
});
