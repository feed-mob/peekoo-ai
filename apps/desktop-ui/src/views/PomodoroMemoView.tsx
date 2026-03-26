import { useState, useEffect, useRef } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { X, Send } from "lucide-react";
import { pomodoroSaveMemo } from "@/features/pomodoro/tool-client";

export default function PomodoroMemoView() {
  const [memo, setMemo] = useState("");
  const [isSaving, setIsSaving] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const handleSave = async () => {
    setIsSaving(true);
    try {
      await pomodoroSaveMemo(null, memo);
      await getCurrentWebviewWindow().close();
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
  }, []);

  return (
    <div className="relative w-full h-screen flex flex-col bg-glass backdrop-blur-2xl border border-glass-border rounded-panel overflow-hidden shadow-panel">
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

      {/* Memo Input */}
      <div className="flex-1 px-4 pb-4">
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
