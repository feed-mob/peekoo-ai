import { invoke } from "@tauri-apps/api/core";

export type PomodoroState = "Idle" | "Running" | "Paused" | "Completed";

export interface PomodoroStatus {
  mode: "work" | "break";
  state: PomodoroState;
  minutes: number;
  time_remaining_secs: number;
  completed_focus: number;
  completed_breaks: number;
  enable_memo: boolean;
  default_work_minutes: number;
  default_break_minutes: number;
}

export interface PomodoroHistoryEntry {
  id: string;
  mode: "work" | "break";
  planned_minutes: number;
  actual_elapsed_secs: number;
  outcome: "completed" | "cancelled";
  started_at: string;
  ended_at: string;
  memo_requested: boolean;
}

type InvokeFn = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

async function callPomodoro<T>(
  command: string,
  args: Record<string, unknown> = {},
  invokeFn: InvokeFn = invoke,
): Promise<T> {
  return invokeFn<T>(command, args);
}

export function getPomodoroStatus(invokeFn?: InvokeFn) {
  return callPomodoro<PomodoroStatus>("pomodoro_get_status", {}, invokeFn);
}

export function getPomodoroHistory(limit: number, invokeFn?: InvokeFn) {
  return callPomodoro<PomodoroHistoryEntry[]>("pomodoro_history", { limit }, invokeFn);
}

export function setPomodoroSettings(
  settings: { work_minutes: number; break_minutes: number; enable_memo: boolean },
  invokeFn?: InvokeFn,
) {
  return callPomodoro<PomodoroStatus>(
    "pomodoro_set_settings",
    {
      workMinutes: settings.work_minutes,
      breakMinutes: settings.break_minutes,
      enableMemo: settings.enable_memo,
    },
    invokeFn,
  );
}

export function startPomodoro(
  input: { mode: "work" | "break"; minutes: number },
  invokeFn?: InvokeFn,
) {
  return callPomodoro<PomodoroStatus>("pomodoro_start", input, invokeFn);
}

export function pausePomodoro(invokeFn?: InvokeFn) {
  return callPomodoro<PomodoroStatus>("pomodoro_pause", {}, invokeFn);
}

export function resumePomodoro(invokeFn?: InvokeFn) {
  return callPomodoro<PomodoroStatus>("pomodoro_resume", {}, invokeFn);
}

export function finishPomodoro(invokeFn?: InvokeFn) {
  return callPomodoro<PomodoroStatus>("pomodoro_finish", {}, invokeFn);
}

export function switchPomodoroMode(mode: "work" | "break", invokeFn?: InvokeFn) {
  return callPomodoro<PomodoroStatus>("pomodoro_switch_mode", { mode }, invokeFn);
}
