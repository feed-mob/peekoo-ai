import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { emitPetReaction } from "@/lib/pet-events";
import { TimerDisplay } from "./TimerDisplay";
import { TimerControls } from "./TimerControls";
import { callPomodoroTool } from "./tool-client";

type PomodoroState = "Idle" | "Running" | "Paused" | "Completed";

interface PomodoroStatus {
  mode: "work" | "break";
  state: PomodoroState;
  minutes: number;
  time_remaining_secs: number;
  completed_sessions: number;
  // Local persistence config from backend
  default_work_minutes: number;
  default_break_minutes: number;
}

const callTool = async (
  toolName: string,
  args: Record<string, unknown> = {},
): Promise<PomodoroStatus | null> => callPomodoroTool<PomodoroStatus>(invoke, toolName, args);

export function PomodoroPanel() {
  const [status, setStatus] = useState<PomodoroStatus | null>(null);
  const [workDuration, setWorkDuration] = useState<number | "">(25);
  const [breakDuration, setBreakDuration] = useState<number | "">(5);
  const [isInitialized, setIsInitialized] = useState(false);

  const fetchStatus = useCallback(async () => {
    const s = await callTool("pomodoro_get_status");
    if (s && s.state !== undefined) {
      setStatus(s);
      // Sync local state once on first load
      if (!isInitialized) {
        setWorkDuration(s.default_work_minutes);
        setBreakDuration(s.default_break_minutes);
        setIsInitialized(true);
      }
    }
  }, [isInitialized]);

  useEffect(() => {
    void fetchStatus();
    const interval = window.setInterval(fetchStatus, 1000);
    return () => window.clearInterval(interval);
  }, [fetchStatus]);

  // Sync settings back to backend when they change
  const persistSettings = useCallback(async (work: number, breakM: number) => {
    await callTool("pomodoro_set_settings", { 
        work_minutes: work, 
        break_minutes: breakM 
    });
    void fetchStatus();
  }, [fetchStatus]);

  const handleWorkDurationChange = (raw: string) => {
    if (raw === "") {
      setWorkDuration("");
      return;
    }
    const val = parseInt(raw);
    if (!isNaN(val)) {
      setWorkDuration(val);
      if (val > 0) {
        void persistSettings(val, breakDuration as number || 1);
      }
    }
  };

  const handleBreakDurationChange = (raw: string) => {
    if (raw === "") {
      setBreakDuration("");
      return;
    }
    const val = parseInt(raw);
    if (!isNaN(val)) {
      setBreakDuration(val);
      if (val > 0) {
        void persistSettings(workDuration as number || 1, val);
      }
    }
  };

  const toggleTimer = useCallback(async () => {
    if (!status) return;
    
    if (status.state === "Running") {
      await callTool("pomodoro_pause");
      void emitPetReaction("pomodoro-break");
    } else {
      if (status.state === "Paused") {
        await callTool("pomodoro_resume");
      } else {
        const minutes = (status.mode === "work" ? workDuration : breakDuration) as number || 1;
        await callTool("pomodoro_start", { mode: status.mode, minutes });
      }
      
      if (status.mode === "work") {
        void emitPetReaction("pomodoro-started", { sticky: true });
      } else {
        void emitPetReaction("pomodoro-resting", { sticky: true });
      }
    }
    void fetchStatus();
  }, [status, fetchStatus]);

  const resetTimer = useCallback(async () => {
    await callTool("pomodoro_finish");
    void emitPetReaction("pomodoro-break");
    void fetchStatus();
  }, [fetchStatus]);

  const switchMode = useCallback(async () => {
    if (!status) return;
    const newMode = status.mode === "work" ? "break" : "work";
    const minutes = newMode === "work" ? workDuration : breakDuration;
    // We finish the current timer and update internal mode state.
    // Our switch to a new mode should also respect the latest user-defined duration.
    await callTool("pomodoro_finish");
    await callTool("pomodoro_start", { mode: newMode, minutes });
    await callTool("pomodoro_pause");
    await callTool("pomodoro_finish"); 
    
    if (newMode === "break") {
        void emitPetReaction("pomodoro-break");
    }
    void fetchStatus();
  }, [status, fetchStatus, workDuration, breakDuration]);

  const formatTime = (seconds: number) => {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
  };

  const getStatusText = () => {
    if (!status) return "Loading...";
    if (status.state === "Idle" || status.state === "Completed") {
      return status.mode === "work" ? "Ready to focus!" : "Take a break!";
    }
    if (status.state === "Running") return status.mode === "work" ? "Focusing..." : "Resting...";
    if (status.state === "Paused") return "Paused";
    return "";
  };

  if (!status) {
    return <div className="flex p-4 text-text-muted">Loading Pomodoro Engine...</div>;
  }

  const isActive = status.state === "Running";
  const maxTime = status.minutes * 60;
  const progress = ((maxTime - status.time_remaining_secs) / maxTime) * 100;

  return (
    <div className="flex flex-col items-center">
      <TimerDisplay
        time={formatTime(status.time_remaining_secs)}
        status={getStatusText()}
        progress={progress}
        isWorkMode={status.mode === "work"}
      />

      <TimerControls
        isActive={isActive}
        onToggle={toggleTimer}
        onReset={resetTimer}
        onSwitchMode={switchMode}
        mode={status.mode}
        completedSessions={status.completed_sessions}
      />

      {status.state === "Idle" && (
        <div className="mt-6 w-full px-8 space-y-4">
          <div className="flex flex-col gap-1.5">
            <label className="text-xs font-medium text-text-muted uppercase tracking-wider">
              Focus (min)
            </label>
            <input
              type="number"
              value={workDuration}
              onChange={(e) => handleWorkDurationChange(e.target.value)}
              className="bg-layer-deep border border-accent-teal/10 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-accent-teal/50 transition-all font-mono"
            />
          </div>
          <div className="flex flex-col gap-1.5">
            <label className="text-xs font-medium text-text-muted uppercase tracking-wider">
              Break (min)
            </label>
            <input
              type="number"
              value={breakDuration}
              onChange={(e) => handleBreakDurationChange(e.target.value)}
              className="bg-layer-deep border border-accent-teal/10 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-accent-teal/50 transition-all font-mono"
            />
          </div>
        </div>
      )}
    </div>
  );
}
