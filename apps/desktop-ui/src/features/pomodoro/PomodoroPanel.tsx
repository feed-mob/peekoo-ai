import { useState, useEffect, useCallback } from "react";
import { emitPetReaction } from "@/lib/pet-events";
import { TimerDisplay } from "./TimerDisplay";
import { TimerControls } from "./TimerControls";

const WORK_MINUTES = 25;
const BREAK_MINUTES = 5;

export function PomodoroPanel() {
  const [timeLeft, setTimeLeft] = useState(WORK_MINUTES * 60);
  const [isActive, setIsActive] = useState(false);
  const [mode, setMode] = useState<"work" | "break">("work");
  const [completedSessions, setCompletedSessions] = useState(0);

  const formatTime = (seconds: number) => {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
  };

  const toggleTimer = useCallback(() => {
    const nextIsActive = !isActive;
    setIsActive(nextIsActive);

    if (nextIsActive) {
      if (mode === "work") {
        void emitPetReaction("pomodoro-started", { sticky: true });
      } else {
        void emitPetReaction("pomodoro-resting", { sticky: true });
      }
    } else {
      void emitPetReaction("pomodoro-break");
    }
  }, [isActive, mode]);

  const resetTimer = useCallback(() => {
    setIsActive(false);
    setTimeLeft(mode === "work" ? WORK_MINUTES * 60 : BREAK_MINUTES * 60);
    void emitPetReaction("pomodoro-break");
  }, [mode]);

  const switchMode = useCallback(() => {
    const newMode = mode === "work" ? "break" : "work";
    setMode(newMode);
    setTimeLeft(newMode === "work" ? WORK_MINUTES * 60 : BREAK_MINUTES * 60);
    setIsActive(false);

    if (newMode === "break") {
      void emitPetReaction("pomodoro-break");
    }
  }, [mode]);

  useEffect(() => {
    let interval: number | null = null;

    if (isActive && timeLeft > 0) {
      interval = window.setInterval(() => {
        setTimeLeft((prev) => prev - 1);
      }, 1000);
    } else if (timeLeft === 0) {
      setIsActive(false);
      if (mode === "work") {
        setCompletedSessions((prev) => prev + 1);
        void emitPetReaction("pomodoro-completed");
      }
    }

    return () => {
      if (interval) clearInterval(interval);
    };
  }, [isActive, timeLeft, mode]);

  const getStatusText = () => {
    if (!isActive && timeLeft === (mode === "work" ? WORK_MINUTES * 60 : BREAK_MINUTES * 60)) {
      return mode === "work" ? "Ready to focus!" : "Take a break!";
    }
    if (isActive) return mode === "work" ? "Focusing..." : "Resting...";
    return mode === "work" ? "Focus session complete!" : "Break complete!";
  };

  const maxTime = mode === "work" ? WORK_MINUTES * 60 : BREAK_MINUTES * 60;
  const progress = ((maxTime - timeLeft) / maxTime) * 100;

  return (
    <div className="flex flex-col items-center">
      <TimerDisplay
        time={formatTime(timeLeft)}
        status={getStatusText()}
        progress={progress}
        isWorkMode={mode === "work"}
      />

      <TimerControls
        isActive={isActive}
        onToggle={toggleTimer}
        onReset={resetTimer}
        onSwitchMode={switchMode}
        mode={mode}
        completedSessions={completedSessions}
      />
    </div>
  );
}
