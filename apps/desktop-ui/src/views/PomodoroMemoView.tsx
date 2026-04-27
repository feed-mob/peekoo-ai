import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { X, Send } from "lucide-react";
import { pomodoroSaveMemo } from "@/features/pomodoro/tool-client";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { Task } from "@/types/task";
import { isMacOsPlatform } from "@/lib/window-transparency";

function taskSortBucket(task: Task): number {
  if (task.status === "in_progress") return 0;
  if (task.status === "todo") return 1;
  return 2;
}

function taskTimeRank(task: Task): number {
  const primary = Date.parse(task.updated_at ?? task.created_at);
  if (!Number.isNaN(primary)) return primary;

  const fallback = Date.parse(task.created_at);
  return Number.isNaN(fallback) ? 0 : fallback;
}

function scheduleProximityRank(task: Task, nowMs: number): number {
  if (!task.scheduled_start_at) return Number.POSITIVE_INFINITY;

  const scheduledMs = Date.parse(task.scheduled_start_at);
  if (Number.isNaN(scheduledMs)) return Number.POSITIVE_INFINITY;

  return Math.abs(scheduledMs - nowMs);
}

export function prepareMemoTaskChoices(
  tasks: Task[],
  nowMs: number = Date.now(),
): {
  tasks: Task[];
  defaultTaskId: string | null;
} {
  const sorted = tasks
    .filter((task) => task.status !== "done")
    .sort((a: Task, b: Task) => {
      const bucketDiff = taskSortBucket(a) - taskSortBucket(b);
      if (bucketDiff !== 0) return bucketDiff;

      const scheduleDiff = scheduleProximityRank(a, nowMs) - scheduleProximityRank(b, nowMs);
      if (scheduleDiff !== 0) return scheduleDiff;

      const timeDiff = taskTimeRank(b) - taskTimeRank(a);
      if (timeDiff !== 0) return timeDiff;

      return a.title.localeCompare(b.title);
    });

  return {
    tasks: sorted,
    defaultTaskId: sorted[0]?.id ?? null,
  };
}

type SubmitPomodoroMemoInput = {
  memo: string;
  taskId: string | null;
  saveMemo: (id: string | null, memo: string, taskId: string | null) => Promise<unknown>;
  closeWindow: () => Promise<unknown>;
};

export async function submitPomodoroMemo({
  memo,
  taskId,
  saveMemo,
  closeWindow,
}: SubmitPomodoroMemoInput) {
  await saveMemo(null, memo, taskId);

  await closeWindow();
}

export default function PomodoroMemoView() {
  const shellClassName = isMacOsPlatform()
    ? "relative w-full h-screen flex flex-col bg-glass/45 border border-glass-border/80 rounded-panel overflow-hidden shadow-panel"
    : "relative w-full h-screen flex flex-col bg-glass backdrop-blur-2xl border border-glass-border rounded-panel overflow-hidden shadow-panel";
  const [memo, setMemo] = useState("");
  const [tasks, setTasks] = useState<Task[]>([]);
  const [selectedTaskId, setSelectedTaskId] = useState<string | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const handleSave = async () => {
    setIsSaving(true);
    try {
      await submitPomodoroMemo({
        memo,
        taskId: selectedTaskId,
        saveMemo: pomodoroSaveMemo,
        closeWindow: () => getCurrentWebviewWindow().close(),
      });
    } catch (err) {
      console.error("Failed to save memo:", err);
      setIsSaving(false);
    }
  };

  const handleSkip = async () => {
    await getCurrentWebviewWindow().close();
  };

  useEffect(() => {
    textareaRef.current?.focus();

    void invoke<Task[]>("list_tasks")
      .then((loadedTasks) => {
        const choices = prepareMemoTaskChoices(loadedTasks);
        setTasks(choices.tasks);
        setSelectedTaskId((prev) => prev ?? choices.defaultTaskId);
      })
      .catch((err) => {
        console.error("Failed to load tasks for pomodoro memo:", err);
      });
  }, []);

  return (
    <div className={shellClassName}>
      {/* Title bar */}
      <div className="relative flex items-center justify-between h-12 px-4 select-none shrink-0">
        <div
          data-tauri-drag-region
          className="absolute inset-0 right-12 cursor-grab active:cursor-grabbing"
        />
        <span className="relative z-10 text-sm font-semibold text-text-primary opacity-80 tracking-wide pointer-events-none">
          Focus Memo
        </span>
        <button
          onClick={handleSkip}
          disabled={isSaving}
          className="relative z-10 p-1.5 rounded-full hover:bg-space-surface text-text-muted hover:text-text-primary transition-colors cursor-pointer"
        >
          <X size={16} />
        </button>
      </div>

      <div className="px-4 pb-3">
        <div className="mb-2 text-[10px] font-extrabold uppercase tracking-[0.18em] text-text-muted">
          Link to Task
        </div>
        <Select value={selectedTaskId ?? "__none__"} onValueChange={(value) => setSelectedTaskId(value === "__none__" ? null : value)}>
          <SelectTrigger className="h-10 rounded-2xl bg-space-deep/60 border-glass-border text-sm">
            <SelectValue placeholder="No task selected" />
          </SelectTrigger>
          <SelectContent className="bg-space-deep border-glass-border">
            <SelectItem value="__none__">No task selected</SelectItem>
            {tasks.map((task) => (
              <SelectItem key={task.id} value={task.id}>
                {task.title}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      {/* Memo Input */}
      <div className="flex-1 px-4 pb-4 min-h-0">
        <textarea
          ref={textareaRef}
          value={memo}
          onChange={(e) => setMemo(e.target.value)}
          placeholder="Have a nice day! :D"
          className="w-full h-full bg-space-deep/60 border border-glass-border rounded-2xl p-4 text-sm text-text-primary placeholder:text-text-muted/50 focus:outline-none focus:border-pomodoro-focus/40 focus:ring-2 focus:ring-pomodoro-focus/20 custom-scrollbar resize-none transition-all"
        />
      </div>

      {/* Actions */}
      <div className="flex justify-end gap-3 px-4 pb-4">
        <button
          onClick={handleSkip}
          disabled={isSaving}
          className="px-5 py-2 rounded-xl text-sm font-medium text-text-muted hover:text-text-primary hover:bg-space-surface/50 transition-all"
        >
          Skip
        </button>
        <button
          onClick={handleSave}
          disabled={isSaving || !memo.trim()}
          className="flex items-center gap-2 px-6 py-2 rounded-xl text-sm font-bold bg-accent-teal text-white hover:bg-accent-teal/90 active:scale-95 disabled:opacity-50 disabled:cursor-not-allowed transition-all shadow-md"
        >
          <Send size={16} />
          {isSaving ? "Sending..." : "Send"}
        </button>
      </div>
    </div>
  );
}
