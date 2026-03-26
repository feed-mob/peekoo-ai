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
  auto_advance: boolean;
  default_work_minutes: number;
  default_break_minutes: number;
  long_break_minutes: number;
  long_break_interval: number;
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
  memo?: string;
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

export function getPomodoroHistoryByDateRange(
  startDate: string,
  endDate: string,
  limit: number,
  invokeFn?: InvokeFn,
) {
  return callPomodoro<PomodoroHistoryEntry[]>(
    "pomodoro_history_by_date_range",
    { startDate, endDate, limit },
    invokeFn,
  );
}

export function setPomodoroSettings(
  settings: {
    work_minutes: number;
    break_minutes: number;
    long_break_minutes: number;
    long_break_interval: number;
    enable_memo: boolean;
    auto_advance: boolean;
  },
  invokeFn: InvokeFn = invoke,
) {
  return callPomodoro<PomodoroStatus>(
    "pomodoro_set_settings",
    {
      workMinutes: settings.work_minutes,
      breakMinutes: settings.break_minutes,
      longBreakMinutes: settings.long_break_minutes,
      longBreakInterval: settings.long_break_interval,
      enableMemo: settings.enable_memo,
      autoAdvance: settings.auto_advance,
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

export function pomodoroSaveMemo(id: string | null, memo: string, invokeFn?: InvokeFn) {
  return callPomodoro<PomodoroStatus>("pomodoro_save_memo", { id, memo }, invokeFn);
}
