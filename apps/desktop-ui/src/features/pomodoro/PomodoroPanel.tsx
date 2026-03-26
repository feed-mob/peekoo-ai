import { useState, useEffect, useCallback, useRef } from "react";
import { TimerDisplay } from "./TimerDisplay";
import { TimerControls } from "./TimerControls";
import { deriveCountdownSnapshot } from "./countdown";
import { Brain, Coffee, History, Settings2, Notebook, Play, Calendar } from "lucide-react";
import { emitPetReaction } from "@/lib/pet-events";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { cn } from "@/lib/utils";
import {
  finishPomodoro,
  getPomodoroHistory,
  getPomodoroHistoryByDateRange,
  getPomodoroStatus,
  pausePomodoro,
  resumePomodoro,
  setPomodoroSettings,
  startPomodoro,
  switchPomodoroMode,
  pomodoroSaveMemo,
  type PomodoroHistoryEntry,
  type PomodoroStatus,
} from "./tool-client";

const HISTORY_LIMIT = 50;

type DateFilter = "today" | "yesterday" | "last7days" | "last30days" | "recent6";

function getDateRange(filter: DateFilter): { start: string; end: string } | null {
  const now = new Date();
  
  // Helper to format date as YYYY-MM-DD in local timezone
  const formatLocalDate = (date: Date): string => {
    const year = date.getFullYear();
    const month = String(date.getMonth() + 1).padStart(2, '0');
    const day = String(date.getDate()).padStart(2, '0');
    return `${year}-${month}-${day}`;
  };
  
  switch (filter) {
    case "today": {
      const start = formatLocalDate(now);
      return { start, end: start };
    }
    case "yesterday": {
      const yesterday = new Date(now);
      yesterday.setDate(yesterday.getDate() - 1);
      const date = formatLocalDate(yesterday);
      return { start: date, end: date };
    }
    case "last7days": {
      const weekAgo = new Date(now);
      weekAgo.setDate(weekAgo.getDate() - 6);
      return {
        start: formatLocalDate(weekAgo),
        end: formatLocalDate(now),
      };
    }
    case "last30days": {
      const monthAgo = new Date(now);
      monthAgo.setDate(monthAgo.getDate() - 29);
      return {
        start: formatLocalDate(monthAgo),
        end: formatLocalDate(now),
      };
    }
    case "recent6":
      return null;
  }
}

export function PomodoroPanel() {
  const [status, setStatus] = useState<PomodoroStatus | null>(null);
  const [history, setHistory] = useState<PomodoroHistoryEntry[]>([]);
  const [dateFilter, setDateFilter] = useState<DateFilter>("recent6");
  const [statusSyncedAtMs, setStatusSyncedAtMs] = useState(() => Date.now());
  const [nowMs, setNowMs] = useState(() => Date.now());
  const [workDuration, setWorkDuration] = useState<number | "">(25);
  const [breakDuration, setBreakDuration] = useState<number | "">(5);
  const [longBreakDuration, setLongBreakDuration] = useState<number | "">(15);
  const [longBreakInterval, setLongBreakInterval] = useState<number | "">(4);
  const [enableMemo, setEnableMemo] = useState(false);
  const [autoAdvance, setAutoAdvance] = useState(false);
  const [isInitialized, setIsInitialized] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const [appliedRecently, setAppliedRecently] = useState(false);

  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [editingMemo, setEditingMemo] = useState<string>("");
  const prevStateRef = useRef<string | null>(null);

  const handleSaveHistoryMemo = async (id: string) => {
    setIsApplying(true);
    await pomodoroSaveMemo(id, editingMemo);
    setIsApplying(false);
    setExpandedId(null);
    void fetchStatus(true);
  };

  const fetchStatus = useCallback(async (forceSync = false) => {
    const nextStatus = await getPomodoroStatus();
    
    // Fetch history based on date filter
    let nextHistory: PomodoroHistoryEntry[];
    const dateRange = getDateRange(dateFilter);
    console.log('[Pomodoro] Fetching history with filter:', dateFilter, 'dateRange:', dateRange);
    
    if (dateRange) {
      nextHistory = await getPomodoroHistoryByDateRange(dateRange.start, dateRange.end, HISTORY_LIMIT);
    } else {
      nextHistory = await getPomodoroHistory(6);
    }
    
    console.log('[Pomodoro] History fetched:', nextHistory.length, 'records');
    
    if (nextStatus && nextStatus.state !== undefined) {
      // Detect completion state change for celebration animation
      const prevState = prevStateRef.current;
      const isJustCompleted = 
        prevState && 
        prevState !== "Completed" && 
        nextStatus.state === "Completed";

      if (isJustCompleted) {
        // Trigger celebration animation
        void emitPetReaction("pomodoro-completed");
      }

      prevStateRef.current = nextStatus.state;
      setStatus(nextStatus);
      setHistory(nextHistory);
      setStatusSyncedAtMs(Date.now());

      if (!isInitialized || forceSync) {
        setWorkDuration(nextStatus.default_work_minutes || 25);
        setBreakDuration(nextStatus.default_break_minutes || 5);
        setLongBreakDuration(nextStatus.long_break_minutes || 15);
        setLongBreakInterval(nextStatus.long_break_interval || 4);
        setEnableMemo(nextStatus.enable_memo || false);
        setAutoAdvance(nextStatus.auto_advance || false);
        if (!isInitialized) setIsInitialized(true);
      } else if (!showSettings) {
        setEnableMemo(nextStatus.enable_memo || false);
        setAutoAdvance(nextStatus.auto_advance || false);
      }
    }
  }, [isInitialized, showSettings, dateFilter]);

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
      long_break_minutes: typeof longBreakDuration === "number" ? longBreakDuration : 15,
      long_break_interval: typeof longBreakInterval === "number" ? longBreakInterval : 4,
      enable_memo: enableMemo,
      auto_advance: autoAdvance,
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
      void emitPetReaction("pomodoro-break");
    } else if (status.state === "Paused") {
      await resumePomodoro();
      void emitPetReaction(status.mode === "work" ? "pomodoro-started" : "pomodoro-resting", { sticky: true });
    } else {
      const minutes = (status.mode === "work" ? workDuration : breakDuration) as number || 1;
      await startPomodoro({ mode: status.mode, minutes });
      void emitPetReaction(status.mode === "work" ? "pomodoro-started" : "pomodoro-resting", { sticky: true });
    }
    void fetchStatus();
  }, [status, fetchStatus, workDuration, breakDuration]);

  const resetTimer = useCallback(async () => {
    await finishPomodoro();
    void emitPetReaction("pomodoro-break");
    void fetchStatus();
  }, [fetchStatus]);

  const switchMode = useCallback(async () => {
    if (!status) return;
    const newMode = status.mode === "work" ? "break" : "work";
    await switchPomodoroMode(newMode);
    void emitPetReaction(newMode === "work" ? "pomodoro-started" : "pomodoro-resting", { sticky: true });
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

  if (!status) return <div className="p-4 text-center text-text-muted">Loading System...</div>;

  const isActive = status.state === "Running";
  const countdown = deriveCountdownSnapshot(status, statusSyncedAtMs, nowMs);
  const progress = countdown.progress;
  const focusCount = status.completed_focus || 0;
  const breakCount = status.completed_breaks || 0;
  const totalCount = focusCount + breakCount;

  console.log('[Pomodoro] Badge counts:', { focusCount, breakCount, totalCount, status });

  const badgeSize = totalCount > 10 ? "w-4 h-4" : "w-5 h-5";
  const iconSize = totalCount > 10 ? "w-2.5 h-2.5" : "w-3 h-3";

  const isDirty = workDuration !== status.default_work_minutes ||
    breakDuration !== status.default_break_minutes ||
    longBreakDuration !== status.long_break_minutes ||
    longBreakInterval !== status.long_break_interval ||
    enableMemo !== status.enable_memo ||
    autoAdvance !== status.auto_advance;

  return (
    <div className="relative flex flex-col items-center w-full px-4 pt-1 h-full bg-transparent overflow-y-auto custom-scrollbar">
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
            status={status.state === "Running" ? (status.mode === "work" ? "Focusing" : "Resting") : (status.mode === "work" ? "Ready" : "Break")}
            progress={progress}
            isWorkMode={status.mode === "work"}
          />

          <div className="flex flex-wrap items-center justify-center gap-1.5 mb-4 shrink-0 px-2 max-w-[240px]">
            {Array.from({ length: focusCount }).map((_, index) => (
              <div key={`f-${index}`} className={`${badgeSize} shrink-0 flex items-center justify-center rounded-md bg-pomodoro-focus/10 border border-pomodoro-focus/20 shadow-none`}>
                <Brain className={`${iconSize} text-pomodoro-focus shadow-none`} />
              </div>
            ))}
            {Array.from({ length: breakCount }).map((_, index) => (
              <div key={`b-${index}`} className={`${badgeSize} shrink-0 flex items-center justify-center rounded-md bg-pomodoro-rest/10 border border-pomodoro-rest/20 shadow-none`}>
                <Coffee className={`${iconSize} text-pomodoro-rest shadow-none`} />
              </div>
            ))}
            {focusCount === 0 && breakCount === 0 && (
              <span className={`text-[10px] shrink-0 font-extrabold uppercase tracking-[0.2em] opacity-80 ${status.mode === "work" ? "text-pomodoro-focus" : "text-pomodoro-rest"}`}>
                Ready to Start?
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
            <div className="mb-3 flex items-center justify-between">
              <div className="flex items-center gap-2 text-[10px] font-extrabold uppercase tracking-[0.2em] text-text-muted">
                <History className="h-3.5 w-3.5" /> Recent Sessions
              </div>
              <select
                value={dateFilter}
                onChange={(e) => setDateFilter(e.target.value as DateFilter)}
                className="text-[10px] font-bold uppercase tracking-wider bg-white/5 border border-white/10 rounded-lg px-2 py-1 text-text-secondary hover:bg-white/10 focus:outline-none focus:border-white/20 cursor-pointer"
              >
                <option value="recent6">Recent 6</option>
                <option value="today">Today</option>
                <option value="yesterday">Yesterday</option>
                <option value="last7days">Last 7 Days</option>
                <option value="last30days">Last 30 Days</option>
              </select>
            </div>

            {history.length === 0 ? (
              <div className="rounded-2xl border border-dashed border-white/8 bg-white/[0.02] px-3 py-4 text-center text-[11px] text-text-muted">
                No sessions yet. Your recent focus and break cycles will appear here.
              </div>
            ) : (
              <div className="space-y-2">
                {history.map((entry) => {
                  const isWork = entry.mode === "work";
                  const isCompleted = entry.outcome === "completed";
                  const isExpanded = expandedId === entry.id;

                  return (
                    <div
                      key={entry.id}
                      className={cn(
                        "flex flex-col gap-3 rounded-2xl border border-white/6 bg-space-deep/40 px-3 py-2.5 transition-all text-left",
                        isExpanded ? "ring-1 ring-white/10 bg-white/[0.04]" : "hover:bg-white/[0.02]"
                      )}
                    >
                      <button 
                        className="flex items-start justify-between w-full"
                        onClick={() => {
                          if (isExpanded) {
                            setExpandedId(null);
                          } else {
                            setExpandedId(entry.id);
                            setEditingMemo(entry.memo || "");
                          }
                        }}
                      >
                        <div className="min-w-0 flex-1 flex text-left">
                          <div className="flex items-center gap-2">
                            <div className={`flex h-6 w-6 items-center justify-center rounded-lg ${isWork ? "bg-pomodoro-focus/10 text-pomodoro-focus" : "bg-pomodoro-rest/10 text-pomodoro-rest"}`}>
                              {isWork ? <Brain className="h-3.5 w-3.5" /> : <Coffee className="h-3.5 w-3.5" />}
                            </div>
                            <div className="min-w-0">
                              <div className="text-[12px] font-bold text-text-primary">
                                {isWork ? "Focus" : "Break"}
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
                      </button>

                      {isExpanded && (
                        <div className="pt-2 border-t border-white/5 animate-in slide-in-from-top-2 duration-200">
                           <div className="flex flex-col gap-2">
                             <div className="text-[10px] font-extrabold uppercase tracking-[0.2em] text-text-muted flex items-center gap-1.5">
                               <Notebook className="w-3 h-3" /> Memo
                             </div>
                             <textarea
                               className="w-full bg-black/20 border border-white/10 rounded-xl p-2.5 text-xs text-text-primary focus:outline-none focus:border-white/20 custom-scrollbar resize-none"
                               rows={3}
                               placeholder={isWork ? "What did you accomplish?" : "How was your break?"}
                               value={editingMemo}
                               onChange={(e) => setEditingMemo(e.target.value)}
                             />
                             <div className="flex justify-end mt-1">
                               <Button
                                 size="sm"
                                 disabled={isApplying || editingMemo === (entry.memo || "")}
                                 onClick={(e) => {
                                  e.stopPropagation();
                                  handleSaveHistoryMemo(entry.id);
                                 }}
                                 className="h-7 text-[10px] rounded-lg px-3 uppercase tracking-wider font-bold bg-white/10 hover:bg-white/20 text-white"
                               >
                                 {isApplying ? "Saving..." : "Save"}
                               </Button>
                             </div>
                           </div>
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        </>
      ) : (
        <div className="w-full mt-12 bg-space-deep/60 backdrop-blur-3xl rounded-3xl border border-glass-border p-5 space-y-5 animate-none shadow-sm">
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <div className="flex items-center gap-2 text-[9px] font-extrabold text-pomodoro-focus uppercase tracking-[0.25em]">
                <Brain className="w-3 h-3" /> Focus
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
                <Coffee className="w-3 h-3" /> Break
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

          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <div className="flex items-center gap-2 text-[9px] font-extrabold text-pomodoro-rest uppercase tracking-[0.25em]">
                <Coffee className="w-3 h-3" /> Long Break
              </div>
              <div className="flex items-center gap-2 bg-white/[0.03] rounded-2xl border border-white/5 p-1.5 focus-within:border-pomodoro-rest/30 transition-none">
                <input
                  type="number"
                  value={longBreakDuration}
                  onChange={(e) => setLongBreakDuration(e.target.value === "" ? "" : parseInt(e.target.value))}
                  className="w-full bg-transparent text-center text-xs font-mono font-bold text-text-primary focus:outline-none [appearance:textfield]"
                />
              </div>
            </div>
            <div className="space-y-2">
              <div className="flex items-center gap-2 text-[9px] font-extrabold text-text-muted uppercase tracking-[0.25em]">
                <History className="w-3 h-3" /> Cycle
              </div>
              <div className="flex items-center gap-2 bg-white/[0.03] rounded-2xl border border-white/5 p-1.5 focus-within:border-white/20 transition-none">
                <input
                  type="number"
                  value={longBreakInterval}
                  onChange={(e) => setLongBreakInterval(e.target.value === "" ? "" : parseInt(e.target.value))}
                  className="w-full bg-transparent text-center text-xs font-mono font-bold text-text-primary focus:outline-none [appearance:textfield]"
                />
              </div>
            </div>
          </div>

          <div className="space-y-1.5 border-t border-white/5 pt-4">
            <div className="flex items-center justify-between py-1 px-1">
              <div className="flex items-center gap-3">
                <div className="p-2 rounded-xl bg-white/5 text-accent-teal/80">
                  <Notebook className="w-3.5 h-3.5" />
                </div>
                <div className="text-[11px] font-bold text-text-primary/90 tracking-tight">Focus Memo</div>
              </div>
              <Checkbox checked={enableMemo} onCheckedChange={(checked) => setEnableMemo(checked === true)} />
            </div>

            <div className="flex items-center justify-between py-1 px-1">
              <div className="flex items-center gap-3">
                <div className="p-2 rounded-xl bg-white/5 text-success/80">
                  <Play className="w-3.5 h-3.5" />
                </div>
                <div className="text-[11px] font-bold text-text-primary/90 tracking-tight">Autopilot</div>
              </div>
              <Checkbox checked={autoAdvance} onCheckedChange={(checked) => setAutoAdvance(checked === true)} />
            </div>
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
            {isApplying ? "Saving..." : appliedRecently ? "Applied" : isDirty ? "Save Changes" : "Synced"}
          </Button>
        </div>
      )}
    </div>
  );
}
