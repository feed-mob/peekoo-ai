import type { PomodoroStatus } from "./tool-client";

export interface CountdownSnapshot {
  timeRemainingSecs: number;
  progress: number;
}

export function deriveCountdownSnapshot(
  status: PomodoroStatus,
  syncedAtMs: number,
  nowMs: number,
): CountdownSnapshot {
  const elapsedSeconds = Math.max(0, Math.floor((nowMs - syncedAtMs) / 1000));
  const timeRemainingSecs = status.state === "Running"
    ? Math.max(0, status.time_remaining_secs - elapsedSeconds)
    : status.time_remaining_secs;
  const totalSeconds = Math.max(1, status.minutes * 60);
  const progress = ((totalSeconds - timeRemainingSecs) / totalSeconds) * 100;

  return {
    timeRemainingSecs,
    progress,
  };
}
