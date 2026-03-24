import { AnimatePresence, motion } from "framer-motion";
import { AlertCircle, Sparkles } from "lucide-react";
import { Streamdown } from "streamdown";
import type { Message } from "@/types/chat";
import { cn } from "@/lib/utils";
import { useIsDarkMode } from "@/hooks/use-is-dark-mode";
import type { MiniChatReplyDisplayMode } from "@/features/chat/chat-session";

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
}: SpriteMiniChatBubbleProps) {
  const isDark = useIsDarkMode();
  const isError = message?.role === "error";
  const isExpanded = displayMode === "expanded";
  const bubbleText = thinking ? "Thinking..." : message?.text;
  const bubbleKey = thinking ? "thinking" : message?.id;

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
            stiffness: 150
          }}
          className={cn(
            "absolute left-1/2 bottom-[calc(100%-15px)] z-30 -translate-x-1/2 pointer-events-none px-4 py-3 shadow-panel backdrop-blur-2xl border",
            isDark ? "bg-black/80" : "bg-white/80",
            isExpanded ? "w-[300px] rounded-2xl" : "w-[224px] rounded-xl",
            thinking ? "border-glow-cyan/30" : isError ? "border-red-500/30" : "border-white/10"
          )}
        >
          {/* Tail pointing down (left-aligned to match SpriteBubble) */}
          <div 
            className={cn(
              "absolute bottom-[-5px] left-8 w-[10px] h-[5px] backdrop-blur-3xl",
              isDark ? "bg-black/80" : "bg-white/80"
            )}
            style={{ 
              clipPath: 'polygon(0% 0%, 100% 0%, 50% 100%)',
              filter: `drop-shadow(0px 1px 0px ${isDark ? 'rgba(255,255,255,0.1)' : 'rgba(255,255,255,0.2)'})`
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
              "mt-1 text-[12px] leading-[18px] text-text-primary [&_p]:m-0",
              isExpanded
                ? "max-h-[156px] overflow-y-auto pr-1.5"
                : "max-h-[66px] overflow-hidden",
              isDark ? "text-white/90" : "text-slate-800"
            )}
          >
            <Streamdown>{bubbleText ?? ""}</Streamdown>
          </div>
        </motion.div>
      ) : null}
    </AnimatePresence>
  );
}

