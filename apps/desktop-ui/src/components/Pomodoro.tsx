import React, { useState, useEffect, useCallback } from 'react';

const WORK_MINUTES = 25;
const BREAK_MINUTES = 5;

export default function Pomodoro() {
  const [timeLeft, setTimeLeft] = useState(WORK_MINUTES * 60);
  const [isActive, setIsActive] = useState(false);
  const [mode, setMode] = useState<'work' | 'break'>('work');
  const [completedSessions, setCompletedSessions] = useState(0);

  const formatTime = (seconds: number) => {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
  };

  const toggleTimer = () => {
    setIsActive(!isActive);
  };

  const resetTimer = () => {
    setIsActive(false);
    setTimeLeft(mode === 'work' ? WORK_MINUTES * 60 : BREAK_MINUTES * 60);
  };

  const switchMode = () => {
    const newMode = mode === 'work' ? 'break' : 'work';
    setMode(newMode);
    setTimeLeft(newMode === 'work' ? WORK_MINUTES * 60 : BREAK_MINUTES * 60);
    setIsActive(false);
  };

  useEffect(() => {
    let interval: NodeJS.Timeout | null = null;

    if (isActive && timeLeft > 0) {
      interval = setInterval(() => {
        setTimeLeft((prev) => prev - 1);
      }, 1000);
    } else if (timeLeft === 0) {
      setIsActive(false);
      if (mode === 'work') {
        setCompletedSessions((prev) => prev + 1);
        // TODO: Play notification sound
        // TODO: Show notification
      }
    }

    return () => {
      if (interval) clearInterval(interval);
    };
  }, [isActive, timeLeft, mode]);

  const getStatusText = () => {
    if (!isActive && timeLeft === (mode === 'work' ? WORK_MINUTES * 60 : BREAK_MINUTES * 60)) {
      return mode === 'work' ? 'Ready to focus!' : 'Take a break!';
    }
    if (isActive) return mode === 'work' ? 'Focusing...' : 'Resting...';
    return mode === 'work' ? 'Focus session complete!' : 'Break complete!';
  };

  return (
    <div className="pomodoro-section">
      <div className="timer-display">
        <div className="time">{formatTime(timeLeft)}</div>
        <div className="status">{getStatusText()}</div>
      </div>

      <div className="timer-controls">
        <button
          className={isActive ? 'pause' : 'start'}
          onClick={toggleTimer}
        >
          {isActive ? '⏸️ Pause' : '▶️ Start'}
        </button>
        <button className="reset" onClick={resetTimer}>
          🔄 Reset
        </button>
      </div>

      <button className="mode-switch" onClick={switchMode}>
        Switch to {mode === 'work' ? 'Break' : 'Work'}
      </button>

      <div className="timer-info">
        Completed sessions: {completedSessions} 🍅
      </div>
    </div>
  );
}
