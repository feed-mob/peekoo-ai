import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { Send } from "lucide-react";
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
    <div className="flex flex-col h-screen bg-space-void/90 backdrop-blur-3xl overflow-hidden select-none border border-white/5 rounded-[32px]">
      {/* Drag Region */}
      <div data-tauri-drag-region className="absolute inset-0 h-10 z-0" />

      <main className="relative z-10 flex flex-col flex-1 p-6">
        <textarea
          autoFocus
          value={memoText}
          onChange={(e) => setMemoText(e.target.value)}
          placeholder="What would you like to record?..."
          className="flex-1 w-full bg-white/[0.02] rounded-3xl border border-white/5 p-5 text-sm text-text-primary focus:outline-none focus:bg-white/[0.04] transition-all resize-none placeholder:text-text-muted/20 leading-relaxed font-medium shadow-inner"
        />

        <div className="flex gap-3 mt-4">
           <Button 
             variant="ghost" 
             className="flex-1 h-11 text-[10px] font-black text-text-muted hover:bg-white/5 hover:text-text-primary uppercase tracking-[0.2em] transition-none" 
             onClick={handleSkip}
             disabled={isSaving}
           >
             Skip
           </Button>
           <Button 
             className="flex-1 h-11 bg-accent-teal/20 hover:bg-accent-teal/30 text-accent-teal font-black uppercase tracking-[0.2em] text-[10px] transition-none shadow-none" 
             onClick={handleSaveMemo}
             disabled={isSaving}
           >
             <Send className="w-4 h-4 mr-2" /> 
             {isSaving ? "Saving" : "Send"}
           </Button>
        </div>
      </main>
    </div>
  );
}
