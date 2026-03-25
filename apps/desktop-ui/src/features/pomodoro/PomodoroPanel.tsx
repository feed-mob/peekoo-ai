import { useState, useEffect, useCallback } from "react";
import { TimerDisplay } from "./TimerDisplay";
import { TimerControls } from "./TimerControls";
import { deriveCountdownSnapshot } from "./countdown";
import { Brain, Coffee, History, Settings2, Notebook } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { useTranslation } from "react-i18next";
import {
  finishPomodoro,
  getPomodoroHistory,
  getPomodoroStatus,
  pausePomodoro,
  resumePomodoro,
  setPomodoroSettings,
  startPomodoro,
  switchPomodoroMode,
  type PomodoroHistoryEntry,
  type PomodoroStatus,
} from "./tool-client";

const HISTORY_LIMIT = 6;

export function PomodoroPanel() {
  const { t } = useTranslation();
  const [status, setStatus] = useState<PomodoroStatus | null>(null);
  const [history, setHistory] = useState<PomodoroHistoryEntry[]>([]);
  const [statusSyncedAtMs, setStatusSyncedAtMs] = useState(() => Date.now());
  const [nowMs, setNowMs] = useState(() => Date.now());
  const [workDuration, setWorkDuration] = useState<number | "">(25);
  const [breakDuration, setBreakDuration] = useState<number | "">(5);
  const [enableMemo, setEnableMemo] = useState(false);
  const [isInitialized, setIsInitialized] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const [appliedRecently, setAppliedRecently] = useState(false);

  const fetchStatus = useCallback(async (forceSync = false) => {
    const [nextStatus, nextHistory] = await Promise.all([
      getPomodoroStatus(),
      getPomodoroHistory(HISTORY_LIMIT),
    ]);
    if (nextStatus && nextStatus.state !== undefined) {
      setStatus(nextStatus);
      setHistory(nextHistory);
      setStatusSyncedAtMs(Date.now());

      if (!isInitialized || forceSync) {
        setWorkDuration(nextStatus.default_work_minutes || 25);
        setBreakDuration(nextStatus.default_break_minutes || 5);
        setEnableMemo(nextStatus.enable_memo || false);
        if (!isInitialized) setIsInitialized(true);
      } else if (!showSettings) {
        setEnableMemo(nextStatus.enable_memo || false);
      }
    }
  }, [isInitialized, showSettings]);

  useEffect(() => {
    void fetchStatus();
    const interval = window.setInterval(fetchStatus, 3000);
    return () => window.clearInterval(interval);
  }, [fetchStatus]);

  useEffect(() => {
    const interval = window.setInterval(() => setNowMs(Date.now()), 1000);
    return () => window.clearInterval(interval);
  }, []);

  const handleApplySettings = async () => {
    setIsApplying(true);
    const result = await setPomodoroSettings({
      work_minutes: typeof workDuration === "number" ? workDuration : 25,
      break_minutes: typeof breakDuration === "number" ? breakDuration : 5,
      enable_memo: enableMemo,
    });
    if (result) setStatus(result);
    setIsApplying(false);
    setAppliedRecently(true);
    setTimeout(() => setAppliedRecently(false), 2000);
  };

  const toggleTimer = useCallback(async () => {
    if (!status) return;
    if (status.state === "Running") {
      await pausePomodoro();
    } else if (status.state === "Paused") {
      await resumePomodoro();
    } else {
      const minutes = (status.mode === "work" ? workDuration : breakDuration) as number || 1;
      await startPomodoro({ mode: status.mode, minutes });
    }
    void fetchStatus();
  }, [status, fetchStatus, workDuration, breakDuration]);

  const resetTimer = useCallback(async () => {
    await finishPomodoro();
    void fetchStatus();
  }, [fetchStatus]);

  const switchMode = useCallback(async () => {
    if (!status) return;
    const newMode = status.mode === "work" ? "break" : "work";
    await switchPomodoroMode(newMode);
    void fetchStatus();
  }, [status, fetchStatus]);

  const formatTime = (seconds: number) => {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
  };

  const formatHistoryTime = (entry: PomodoroHistoryEntry) => {
    const actualMinutes = Math.floor(entry.actual_elapsed_secs / 60);
    const actualSeconds = entry.actual_elapsed_secs % 60;
    return `${actualMinutes}m ${actualSeconds.toString().padStart(2, "0")}s / ${entry.planned_minutes}m`;
  };

  const formatHistoryTimestamp = (value: string) => {
    const date = new Date(value);
    return Number.isNaN(date.getTime())
      ? value
      : date.toLocaleString([], {
          month: "short",
          day: "numeric",
          hour: "numeric",
          minute: "2-digit",
        });
  };

  if (!status) {
    return <div className="p-4 text-center text-text-muted">{t("pomodoro.loadingSystem")}</div>;
  }

  const isActive = status.state === "Running";
  const countdown = deriveCountdownSnapshot(status, statusSyncedAtMs, nowMs);
  const progress = countdown.progress;
  const focusCount = status.completed_focus || 0;
  const breakCount = status.completed_breaks || 0;
  const totalCount = focusCount + breakCount;

  const badgeSize = totalCount > 10 ? "w-4 h-4" : "w-5 h-5";
  const iconSize = totalCount > 10 ? "w-2.5 h-2.5" : "w-3 h-3";

  const isDirty = workDuration !== status.default_work_minutes ||
    breakDuration !== status.default_break_minutes ||
    enableMemo !== status.enable_memo;

  return (
    <div className="relative flex flex-col items-center w-full px-4 pt-1 h-full">
      <button
        onClick={() => {
          if (showSettings) setShowSettings(false);
          else {
            void fetchStatus(true);
            setShowSettings(true);
          }
        }}
        className={`absolute right-4 top-0 p-2 rounded-full transition-all z-20 ${showSettings ? "bg-accent-teal/10 text-accent-teal shadow-none border border-accent-teal/20" : "hover:bg-white/5 text-text-muted"}`}
      >
        <Settings2 className="w-4 h-4" />
      </button>

      {!showSettings ? (
        <>
          <TimerDisplay
            time={formatTime(countdown.timeRemainingSecs)}
            status={
              status.state === "Running"
                ? status.mode === "work"
                  ? t("pomodoro.status.focusing")
                  : t("pomodoro.status.resting")
                : status.mode === "work"
                  ? t("pomodoro.status.ready")
                  : t("pomodoro.status.break")
            }
            progress={progress}
            isWorkMode={status.mode === "work"}
          />

          <div className="flex flex-wrap items-center justify-center gap-1.5 mb-2 min-h-[24px] px-2 max-w-[240px]">
            {Array.from({ length: focusCount }).map((_, index) => (
              <div key={`f-${index}`} className={`${badgeSize} flex items-center justify-center rounded-md bg-pomodoro-focus/10 border border-pomodoro-focus/20 shadow-none`}>
                <Brain className={`${iconSize} text-pomodoro-focus shadow-none`} />
              </div>
            ))}
            {Array.from({ length: breakCount }).map((_, index) => (
              <div key={`b-${index}`} className={`${badgeSize} flex items-center justify-center rounded-md bg-pomodoro-rest/10 border border-pomodoro-rest/20 shadow-none`}>
                <Coffee className={`${iconSize} text-pomodoro-rest shadow-none`} />
              </div>
            ))}
            {focusCount === 0 && breakCount === 0 && (
              <span className={`text-[10px] font-extrabold uppercase tracking-[0.2em] opacity-80 ${status.mode === "work" ? "text-pomodoro-focus" : "text-pomodoro-rest"}`}>
                {t("pomodoro.readyToStart")}
              </span>
            )}
          </div>

          <TimerControls
            isActive={isActive}
            onToggle={toggleTimer}
            onReset={resetTimer}
            onSwitchMode={switchMode}
            mode={status.mode}
          />

          <div className="mt-5 w-full rounded-3xl border border-white/6 bg-white/[0.03] p-4 shadow-none">
            <div className="mb-3 flex items-center gap-2 text-[10px] font-extrabold uppercase tracking-[0.2em] text-text-muted">
              <History className="h-3.5 w-3.5" /> {t("pomodoro.recentSessions")}
            </div>

            {history.length === 0 ? (
              <div className="rounded-2xl border border-dashed border-white/8 bg-white/[0.02] px-3 py-4 text-center text-[11px] text-text-muted">
                {t("pomodoro.noSessions")}
              </div>
            ) : (
              <div className="space-y-2">
                {history.map((entry) => {
                  const isWork = entry.mode === "work";
                  const isCompleted = entry.outcome === "completed";
                  return (
                    <div
                      key={entry.id}
                      className="flex items-start justify-between gap-3 rounded-2xl border border-white/6 bg-space-deep/40 px-3 py-2.5"
                    >
                      <div className="min-w-0 flex-1">
                        <div className="flex items-center gap-2">
                          <div className={`flex h-6 w-6 items-center justify-center rounded-lg ${isWork ? "bg-pomodoro-focus/10 text-pomodoro-focus" : "bg-pomodoro-rest/10 text-pomodoro-rest"}`}>
                            {isWork ? <Brain className="h-3.5 w-3.5" /> : <Coffee className="h-3.5 w-3.5" />}
                          </div>
                          <div className="min-w-0">
                            <div className="text-[12px] font-bold text-text-primary">
                              {isWork ? t("pomodoro.mode.focus") : t("pomodoro.mode.break")}
                            </div>
                            <div className="text-[10px] text-text-muted">
                              {formatHistoryTimestamp(entry.ended_at)}
                            </div>
                          </div>
                        </div>
                      </div>

                      <div className="shrink-0 text-right">
                        <div className={`text-[10px] font-extrabold uppercase tracking-[0.16em] ${isCompleted ? "text-success" : "text-text-muted"}`}>
                          {entry.outcome}
                        </div>
                        <div className="mt-1 text-[11px] font-mono text-text-secondary">
                          {formatHistoryTime(entry)}
                        </div>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        </>
      ) : (
        <div className="w-full mt-10 bg-space-deep/60 backdrop-blur-3xl rounded-3xl border border-glass-border p-5 space-y-5 animate-none shadow-sm">
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <div className="flex items-center gap-2 text-[9px] font-extrabold text-pomodoro-focus uppercase tracking-[0.25em]">
                <Brain className="w-3 h-3" /> {t("pomodoro.mode.focus")}
              </div>
              <div className="flex items-center gap-2 bg-white/[0.03] rounded-2xl border border-white/5 p-1.5 focus-within:border-pomodoro-focus/30 transition-none">
                <input
                  type="number"
                  value={workDuration}
                  onChange={(e) => setWorkDuration(e.target.value === "" ? "" : parseInt(e.target.value))}
                  className="w-full bg-transparent text-center text-xs font-mono font-bold text-text-primary focus:outline-none [appearance:textfield]"
                />
              </div>
            </div>
            <div className="space-y-2">
              <div className="flex items-center gap-2 text-[9px] font-extrabold text-pomodoro-rest uppercase tracking-[0.25em]">
                <Coffee className="w-3 h-3" /> {t("pomodoro.mode.break")}
              </div>
              <div className="flex items-center gap-2 bg-white/[0.03] rounded-2xl border border-white/5 p-1.5 focus-within:border-pomodoro-rest/30 transition-none">
                <input
                  type="number"
                  value={breakDuration}
                  onChange={(e) => setBreakDuration(e.target.value === "" ? "" : parseInt(e.target.value))}
                  className="w-full bg-transparent text-center text-xs font-mono font-bold text-text-primary focus:outline-none [appearance:textfield]"
                />
              </div>
            </div>
          </div>

          <div className="flex items-center justify-between py-1 px-1 border-t border-white/5 pt-4">
            <div className="flex items-center gap-3">
              <div className="p-2 rounded-xl bg-white/5 text-accent-teal/80">
                <Notebook className="w-3.5 h-3.5" />
              </div>
              <div className="text-[11px] font-bold text-text-primary/90 tracking-tight">{t("pomodoro.focusMemo")}</div>
            </div>
            <Checkbox checked={enableMemo} onCheckedChange={(checked) => setEnableMemo(checked === true)} />
          </div>

          <Button
            variant="outline"
            size="sm"
            disabled={!isDirty || isApplying}
            className={`w-full h-11 rounded-2xl text-[10px] uppercase tracking-[0.2em] font-black transition-none shadow-none border-0 ${
              appliedRecently
                ? "bg-success/15 text-success/90"
                : isDirty
                  ? "bg-accent-teal/20 text-accent-teal"
                  : "bg-white/[0.02] text-text-muted opacity-30 cursor-not-allowed"
            }`}
            onClick={handleApplySettings}
          >
            {isApplying
              ? t("pomodoro.actions.saving")
              : appliedRecently
                ? t("pomodoro.actions.applied")
                : isDirty
                  ? t("pomodoro.actions.saveChanges")
                  : t("pomodoro.actions.synced")}
          </Button>
        </div>
      )}
    </div>
  );
}
