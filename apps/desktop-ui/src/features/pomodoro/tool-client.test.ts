import { describe, expect, test } from "bun:test";
import {
  finishPomodoro,
  getPomodoroHistory,
  getPomodoroStatus,
  setPomodoroSettings,
  switchPomodoroMode,
} from "./tool-client.ts";

describe("pomodoro tool client", () => {
  test("loads status from the built-in pomodoro command", async () => {
    const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];

    const invoke = async <T>(command: string, args?: Record<string, unknown>) => {
      calls.push({ command, args });
      return { state: "Idle" } as T;
    };

    const result = await getPomodoroStatus(invoke);

    expect(result).toEqual({ state: "Idle" });
    expect(calls).toEqual([{ command: "pomodoro_get_status", args: {} }]);
  });

  test("passes settings using tauri command arguments", async () => {
    const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];

    const invoke = async <T>(command: string, args?: Record<string, unknown>) => {
      calls.push({ command, args });
      return { ok: true } as T;
    };

    await setPomodoroSettings(
      { work_minutes: 45, break_minutes: 10, enable_memo: true },
      invoke,
    );

    expect(calls).toEqual([
      {
        command: "pomodoro_set_settings",
        args: { workMinutes: 45, breakMinutes: 10, enableMemo: true },
      },
    ]);
  });

  test("uses built-in commands for finish and mode switching", async () => {
    const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];

    const invoke = async <T>(command: string, args?: Record<string, unknown>) => {
      calls.push({ command, args });
      return {} as T;
    };

    await finishPomodoro(invoke);
    await switchPomodoroMode("break", invoke);

    expect(calls).toEqual([
      { command: "pomodoro_finish", args: {} },
      { command: "pomodoro_switch_mode", args: { mode: "break" } },
    ]);
  });

  test("loads recent pomodoro history with a limit", async () => {
    const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];

    const invoke = async <T>(command: string, args?: Record<string, unknown>) => {
      calls.push({ command, args });
      return [{ id: "cycle-1", mode: "work" }] as T;
    };

    const result = await getPomodoroHistory(5, invoke);

    expect(result).toEqual([{ id: "cycle-1", mode: "work" }]);
    expect(calls).toEqual([{ command: "pomodoro_history", args: { limit: 5 } }]);
  });
});
