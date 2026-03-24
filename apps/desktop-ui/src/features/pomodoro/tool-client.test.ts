import { afterEach, describe, expect, mock, test } from "bun:test";
import { callPomodoroTool } from "./tool-client.ts";

const consoleError = mock(() => {});

console.error = consoleError as typeof console.error;

afterEach(() => {
  consoleError.mockClear();
});

describe("callPomodoroTool", () => {
  test("enables the pomodoro plugin and retries when the tool is missing", async () => {
    const calls: Array<{ command: string; args: Record<string, unknown> }> = [];
    let firstToolCall = true;

    const invoke = async (command: string, args: Record<string, unknown>) => {
      calls.push({ command, args });

      if (command === "plugin_call_tool") {
        if (firstToolCall) {
          firstToolCall = false;
          throw new Error("Tool not found: pomodoro_get_status");
        }

        return JSON.stringify({ ok: true });
      }

      if (command === "plugin_enable") {
        return null;
      }

      throw new Error(`Unexpected command: ${command}`);
    };

    const result = await callPomodoroTool(invoke, "pomodoro_get_status");

    expect(result).toEqual({ ok: true });
    expect(calls).toEqual([
      {
        command: "plugin_call_tool",
        args: { toolName: "pomodoro_get_status", argsJson: "{}" },
      },
      {
        command: "plugin_enable",
        args: { pluginKey: "pomodoro" },
      },
      {
        command: "plugin_call_tool",
        args: { toolName: "pomodoro_get_status", argsJson: "{}" },
      },
    ]);
  });

  test("returns null without enabling the plugin for unrelated errors", async () => {
    const calls: Array<{ command: string; args: Record<string, unknown> }> = [];

    const invoke = async (command: string, args: Record<string, unknown>) => {
      calls.push({ command, args });
      throw new Error("permission denied");
    };

    const result = await callPomodoroTool(invoke, "pomodoro_get_status");

    expect(result).toBeNull();
    expect(calls).toEqual([
      {
        command: "plugin_call_tool",
        args: { toolName: "pomodoro_get_status", argsJson: "{}" },
      },
    ]);
    expect(consoleError).toHaveBeenCalledTimes(1);
  });
});
