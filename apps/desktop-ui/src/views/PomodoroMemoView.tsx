import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { Send, X, NotebookPen } from "lucide-react";
import { Button } from "@/components/ui/button";

const callTool = async (toolName: string, args: any = {}): Promise<any> => {
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

export default function PomodoroMemoView() {
  const [memoText, setMemoText] = useState("");
  const [isSaving, setIsSaving] = useState(false);
  const window = getCurrentWebviewWindow();

  const handleSaveMemo = async () => {
    if (memoText.trim()) {
      setIsSaving(true);
      await callTool("pomodoro_add_memo", memoText);
      setIsSaving(false);
    }
    void window.close();
  };

  const handleSkip = () => {
    void window.close();
  };

  return (
    <div className="flex flex-col h-screen bg-space-void/75 backdrop-blur-3xl overflow-hidden select-none border border-black/5 dark:border-white/5 rounded-[28px] shadow-2xl transition-all duration-500">
      {/* Header / Drag Region */}
      <header data-tauri-drag-region className="flex items-center justify-between px-6 py-4 z-20 cursor-default border-b border-black/5 dark:border-white/5">
         <div className="flex items-center gap-3 pointer-events-none">
            <NotebookPen className="w-4 h-4 text-glow-green/40 dark:text-glow-olive/60" />
            <span className="text-[10px] font-black uppercase tracking-[0.25em] text-text-primary/30 dark:text-text-muted/50">Focus Memo</span>
         </div>
         <button 
           onClick={handleSkip}
           className="p-1.5 rounded-full hover:bg-black/5 dark:hover:bg-white/5 text-text-muted/40 hover:text-text-primary transition-all"
         >
           <X size={14} />
         </button>
      </header>

      <main className="relative z-10 flex flex-col flex-1 px-6 pb-6 pt-5">
        <textarea
          autoFocus
          value={memoText}
          onChange={(e) => setMemoText(e.target.value)}
          placeholder="What would you like to capture?..."
          className="flex-1 w-full bg-space-deep/65 dark:bg-space-deep/60 rounded-2xl border border-black/5 dark:border-white/5 p-5 text-sm text-text-primary dark:text-text-secondary focus:outline-none focus:bg-space-overlay/70 dark:focus:bg-space-deep/75 transition-all resize-none placeholder:text-text-muted/20 dark:placeholder:text-text-muted/10 leading-relaxed font-medium shadow-inner"
        />

        <div className="flex gap-3 mt-5">
           <Button 
             variant="ghost" 
             className="flex-1 h-11 text-[10px] font-black text-text-muted/40 dark:text-text-muted/40 hover:bg-black/5 dark:hover:bg-white/5 hover:text-text-primary uppercase tracking-[0.2em] transition-none rounded-xl" 
             onClick={handleSkip}
             disabled={isSaving}
           >
             Skip
           </Button>
           <Button 
             className="flex-1 h-11 bg-space-surface/20 hover:bg-space-surface/30 text-text-primary font-black uppercase tracking-[0.2em] text-[10px] transition-none shadow-none rounded-xl border border-black/5 dark:border-white/10" 
             onClick={handleSaveMemo}
             disabled={isSaving}
           >
             <Send className="w-3.5 h-3.5 mr-2" /> 
             {isSaving ? "Saving" : "Send"}
           </Button>
        </div>
      </main>
    </div>
  );
}
