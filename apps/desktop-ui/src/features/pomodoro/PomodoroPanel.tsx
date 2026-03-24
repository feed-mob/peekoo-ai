import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { emitPetReaction } from "@/lib/pet-events";
import { TimerDisplay } from "./TimerDisplay";
import { TimerControls } from "./TimerControls";
import { Brain, Coffee, Settings2, Notebook } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";

type PomodoroState = "Idle" | "Running" | "Paused" | "Completed";

interface PomodoroStatus {
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

const callTool = async (toolName: string, args: any = {}): Promise<PomodoroStatus | any> => {
  try {
    const res = await invoke("plugin_call_tool", {
      toolName,
      argsJson: JSON.stringify(args)
    });
    return JSON.parse(res as string);
  } catch (err) {
    console.error(`Error calling ${toolName}:`, err);
    return null;
  }
};

export function PomodoroPanel() {
  const [status, setStatus] = useState<PomodoroStatus | null>(null);
  const [workDuration, setWorkDuration] = useState<number | "">(25);
  const [breakDuration, setBreakDuration] = useState<number | "">(5);
  const [enableMemo, setEnableMemo] = useState(false);
  const [isInitialized, setIsInitialized] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const [appliedRecently, setAppliedRecently] = useState(false);

  const fetchStatus = useCallback(async (forceSync = false) => {
    const s: PomodoroStatus = await callTool("pomodoro_get_status");
    if (s && s.state !== undefined) {
      setStatus(s);
      
      if (!isInitialized || forceSync) {
        setWorkDuration(s.default_work_minutes || 25);
        setBreakDuration(s.default_break_minutes || 5);
        setEnableMemo(s.enable_memo || false);
        // Initialize the completion tracker from the current backend value
        // to prevent immediate pop-up if the session was already completed
        if (!isInitialized) setIsInitialized(true);
      } else if (!showSettings) {
        setEnableMemo(s.enable_memo || false);
      }

    }
  }, [isInitialized, showSettings]);

  useEffect(() => {
    void fetchStatus();
    const interval = window.setInterval(fetchStatus, 3000);
    return () => window.clearInterval(interval);
  }, [fetchStatus]);

  const handleApplySettings = async () => {
    setIsApplying(true);
    const result = await callTool("pomodoro_set_settings", { 
        work_minutes: typeof workDuration === 'number' ? workDuration : 25, 
        break_minutes: typeof breakDuration === 'number' ? breakDuration : 5,
        enable_memo: enableMemo
    });
    if (result) setStatus(result);
    setIsApplying(false);
    setAppliedRecently(true);
    setTimeout(() => setAppliedRecently(false), 2000);
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
      void emitPetReaction(status.mode === "work" ? "pomodoro-started" : "pomodoro-resting", { sticky: true });
    }
    void fetchStatus();
  }, [status, fetchStatus, workDuration, breakDuration]);

  const resetTimer = useCallback(async () => {
    await callTool("pomodoro_finish");
    void emitPetReaction("pomodoro-break");
    void fetchStatus();
  }, [fetchStatus]);

  const switchMode = useCallback(async () => {
    if (!status) return;
    const newMode = status.mode === "work" ? "break" : "work";
    await callTool("pomodoro_finish");
    await callTool("pomodoro_start", { mode: newMode, minutes: newMode === "work" ? (workDuration as number || 25) : (breakDuration as number || 5) });
    await callTool("pomodoro_pause");
    await callTool("pomodoro_finish"); 
    void fetchStatus();
  }, [status, fetchStatus, workDuration, breakDuration]);

  const formatTime = (seconds: number) => {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
  };

  if (!status) return <div className="p-4 text-center text-text-muted">Loading System...</div>;

  const isActive = status.state === "Running";
  const progress = ((status.minutes * 60 - status.time_remaining_secs) / (status.minutes * 60)) * 100;
  const focusCount = status.completed_focus || 0;
  const breakCount = status.completed_breaks || 0;
  const totalCount = focusCount + breakCount;
  
  const badgeSize = totalCount > 10 ? "w-4 h-4" : "w-5 h-5";
  const iconSize = totalCount > 10 ? "w-2.5 h-2.5" : "w-3 h-3";

  const isDirty = status && (
    workDuration !== status.default_work_minutes || 
    breakDuration !== status.default_break_minutes ||
    enableMemo !== status.enable_memo
  );

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
        className={`absolute right-4 top-0 p-2 rounded-full transition-all z-20 ${showSettings ? 'bg-accent-teal/10 text-accent-teal shadow-none border border-accent-teal/20' : 'hover:bg-white/5 text-text-muted'}`}
      >
        <Settings2 className="w-4 h-4" />
      </button>

      {!showSettings ? (
        <>
          <TimerDisplay
            time={formatTime(status.time_remaining_secs)}
            status={status.state === "Running" ? (status.mode === "work" ? "Focusing" : "Resting") : (status.mode === "work" ? "Ready" : "Break")}
            progress={progress}
            isWorkMode={status.mode === "work"}
          />

          <div className="flex flex-wrap items-center justify-center gap-1.5 mb-2 min-h-[24px] px-2 max-w-[240px]">
             {Array.from({ length: focusCount }).map((_, i) => (
                <div key={`f-${i}`} className={`${badgeSize} flex items-center justify-center rounded-md bg-pomodoro-focus/10 border border-pomodoro-focus/20 shadow-none`}>
                  <Brain className={`${iconSize} text-pomodoro-focus shadow-none`} />
                </div>
             ))}
             {Array.from({ length: breakCount }).map((_, i) => (
                <div key={`b-${i}`} className={`${badgeSize} flex items-center justify-center rounded-md bg-pomodoro-rest/10 border border-pomodoro-rest/20 shadow-none`}>
                  <Coffee className={`${iconSize} text-pomodoro-rest shadow-none`} />
                </div>
             ))}
            {focusCount === 0 && breakCount === 0 && (
              <span className={`text-[10px] font-extrabold uppercase tracking-[0.2em] opacity-80 ${status.mode === "work" ? "text-pomodoro-focus" : "text-pomodoro-rest"}`}>
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
        </>
      ) : (
        <div className="w-full mt-10 bg-space-deep/60 backdrop-blur-3xl rounded-3xl border border-glass-border p-5 space-y-5 animate-none shadow-sm">
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

           <div className="flex items-center justify-between py-1 px-1 border-t border-white/5 pt-4">
              <div className="flex items-center gap-3">
                 <div className="p-2 rounded-xl bg-white/5 text-accent-teal/80">
                    <Notebook className="w-3.5 h-3.5" />
                 </div>
                 <div className="text-[11px] font-bold text-text-primary/90 tracking-tight">Focus Memo</div>
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
              {isApplying ? (
                "Saving..."
              ) : appliedRecently ? (
                "Applied"
              ) : isDirty ? (
                "Save Changes"
              ) : (
                "Synced"
              )}
           </Button>
        </div>
      )}
    </div>
  );
}
