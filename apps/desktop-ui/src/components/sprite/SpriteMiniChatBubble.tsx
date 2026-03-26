import { useState } from 'react';
import { AnimatePresence, motion } from "framer-motion";
import { AlertCircle, Sparkles } from "lucide-react";
import { Streamdown } from "streamdown";
import type { Message } from "@/types/chat";
import { cn } from "@/lib/utils";
import { useIsDarkMode } from "@/hooks/use-is-dark-mode";
import type { MiniChatReplyDisplayMode } from "@/features/chat/chat-session";
import { pomodoroSaveMemo } from "@/features/pomodoro/tool-client";

interface SpriteMiniChatBubbleProps {
  message: Message | null;
  visible: boolean;
  thinking?: boolean;
  displayMode?: MiniChatReplyDisplayMode;
}

export function SpriteMiniChatBubble({
  message,
  visible,
  thinking = false,
  displayMode = "compact",
}: SpriteMiniChatBubbleProps): JSX.Element | null {
  const isDark = useIsDarkMode();
  const isError = message?.role === "error";
  const isExpanded = displayMode === "expanded";
  const bubbleText = thinking ? "Thinking..." : message?.text;
  const bubbleKey = thinking ? "thinking" : message?.id;
  
  // Quick cast to check for memo role type
  const isMemo = (message as any)?.role === "memo";

  const [memo, setMemo] = useState<string>("");
  const [isSaving, setIsSaving] = useState(false);

  const handleSave = async () => {
    setIsSaving(true);
    // null id means it saves to the latest work cycle
    await pomodoroSaveMemo(null, memo).catch(console.error);
    setIsSaving(false);
    setMemo("");
    // After save, the agent will typically drop the memo ask or we can force close
  };

  const handleCancel = () => {
    setMemo("");
  };

  return (
    <AnimatePresence>
      {(message || thinking) && visible ? (
        <motion.div
          key={bubbleKey}
          initial={{ opacity: 0, scale: 0.95, y: 10 }}
          animate={{ opacity: 1, scale: 1, y: 0 }}
          exit={{ opacity: 0, scale: 0.98, y: 5 }}
          transition={{
            type: "spring",
            damping: 25,
            stiffness: 150,
          }}
          className={cn(
            "absolute left-1/2 bottom-[calc(100%-15px)] z-30 -translate-x-1/2 pointer-events-none px-4 py-3 shadow-panel backdrop-blur-2xl border",
            isDark ? "bg-black/80" : "bg-white/80",
            isExpanded ? "w-[300px] rounded-2xl" : "w-[224px] rounded-xl",
            thinking ? "border-glow-cyan/30" : isError ? "border-red-500/30" : "border-white/10"
          )}
        >
          {/* Tail pointing down */}
          <div
            className={cn(
              "absolute bottom-[-5px] left-8 w-[10px] h-[5px] backdrop-blur-3xl",
              isDark ? "bg-black/80" : "bg-white/80"
            )}
            style={{
              clipPath: 'polygon(0% 0%, 100% 0%, 50% 100%)',
              filter: `drop-shadow(0px 1px 0px ${isDark ? 'rgba(255,255,255,0.1)' : 'rgba(255,255,255,0.2)'})`,
            }}
          />

          <div className={cn(
            "flex items-center gap-1.5 text-[9px] font-semibold uppercase tracking-[0.2em]",
            thinking ? "text-glow-cyan" : isError ? "text-red-400" : "text-glow-cyan/80"
          )}>
            {isError ? <AlertCircle size={10} /> : <Sparkles size={10} className={thinking ? "animate-pulse" : ""} />}
            <span>
              {thinking
                ? "Thinking"
                : isError
                ? "Need attention"
                : isExpanded
                ? "Reading mode"
                : "Quick reply"}
            </span>
          </div>

          <div
            className={cn(
              "mt-1 text-[12px] leading-[18px] text-text-primary [&_p]:m-0 pointer-events-auto",
              isExpanded ? "max-h-[156px] overflow-y-auto pr-1.5" : "max-h-[120px] overflow-hidden",
              isDark ? "text-white/90" : "text-slate-800"
            )}
          >
            <Streamdown>{bubbleText ?? ""}</Streamdown>
            
            {isMemo && (
              <div className="mt-3 flex flex-col gap-2">
                <textarea
                  className={cn(
                    "w-full rounded-md p-2 text-xs border outline-none",
                    isDark 
                      ? "bg-black/40 border-white/10 text-white focus:border-white/30" 
                      : "bg-white/60 border-black/10 text-black focus:border-black/30"
                  )}
                  rows={2}
                  placeholder="记录你的成果..."
                  value={memo}
                  onChange={e => setMemo(e.target.value)}
                />
                <div className="flex justify-end gap-2">
                  <button
                    className={cn(
                      "px-3 py-1 rounded bg-gray-200 text-gray-700 text-[10px] uppercase font-bold tracking-wider hover:bg-gray-300 transition-colors",
                      isDark && "bg-white/10 text-white/80 hover:bg-white/20"
                    )}
                    onClick={handleCancel}
                    disabled={isSaving}
                  >
                    忽略
                  </button>
                  <button
                    className={cn(
                      "px-3 py-1 rounded bg-pomodoro-focus text-white text-[10px] uppercase font-bold tracking-wider hover:bg-pomodoro-focus/90 transition-colors",
                      isDark && "hover:bg-pomodoro-focus/80"
                    )}
                    onClick={handleSave}
                    disabled={isSaving || !memo.trim()}
                  >
                    {isSaving ? "Saving..." : "保存"}
                  </button>
                </div>
              </div>
            )}
          </div>
        </motion.div>
      ) : null}
    </AnimatePresence>
  );
}
