import { AnimatePresence, motion } from "framer-motion";
import { AlertCircle, Sparkles } from "lucide-react";
import { Streamdown } from "streamdown";
import type { Message } from "@/types/chat";
import { cn } from "@/lib/utils";
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
  const isError = message?.role === "error";
  const isExpanded = displayMode === "expanded";
  const bubbleText = thinking ? "Thinking..." : message?.text;
  const bubbleKey = thinking ? "thinking" : message?.id;

  return (
    <AnimatePresence>
      {(message || thinking) && visible ? (
        <motion.div
          key={bubbleKey}
          initial={{ opacity: 0, y: 8, scale: 0.95 }}
          animate={{ opacity: 1, y: 0, scale: 1 }}
          exit={{ opacity: 0, y: 4, scale: 0.97 }}
          transition={{ duration: 0.2, ease: "easeOut" }}
          className={cn(
            "absolute left-1/2 top-2 z-30 -translate-x-1/2 rounded-2xl border border-glass-border bg-glass/95 px-4 py-3 shadow-panel backdrop-blur-xl",
            isExpanded ? "w-[248px]" : "w-[184px]",
          )}
        >
          <div className="absolute bottom-[-7px] left-1/2 h-3.5 w-3.5 -translate-x-1/2 rotate-45 border-b border-r border-glass-border bg-glass/95" />
          <div className="flex items-center gap-1.5 text-[9px] font-semibold uppercase tracking-[0.2em] text-glow-cyan/80">
            {isError ? <AlertCircle size={10} /> : <Sparkles size={10} />}
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
              "mt-1 text-[12px] leading-[16px] text-text-primary [&_p]:m-0",
              isExpanded ? "max-h-[156px] overflow-y-auto pr-1" : "max-h-[66px] overflow-hidden",
            )}
          >
            <Streamdown>{bubbleText ?? ""}</Streamdown>
          </div>
        </motion.div>
      ) : null}
    </AnimatePresence>
  );
}
