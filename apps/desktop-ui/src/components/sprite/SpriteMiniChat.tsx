
import { useEffect, useRef, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { Expand, Send } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import { useIsDarkMode } from "@/hooks/use-is-dark-mode";
import { useTranslation } from "react-i18next";

interface SpriteMiniChatProps {
  open: boolean;
  isTyping: boolean;
  onClose: () => void;
  onOpenFullChat: () => Promise<void>;
  onSubmit: (message: string) => Promise<boolean>;
}

export function SpriteMiniChat({
  open,
  isTyping,
  onClose,
  onOpenFullChat,
  onSubmit,
}: SpriteMiniChatProps) {
  const { t } = useTranslation();
  const isDark = useIsDarkMode();
  const [input, setInput] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!open) {
      setInput("");
      return;
    }

    window.setTimeout(() => inputRef.current?.focus(), 0);
  }, [open]);

  useEffect(() => {
    if (!open) {
      return;
    }

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [onClose, open]);

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!input.trim()) {
      return;
    }

    const didSend = await onSubmit(input);
    if (didSend) {
      setInput("");
    }
  };

  return (
    <AnimatePresence>
      {open ? (
        <motion.div
          initial={{ opacity: 0, y: -10, scale: 0.96 }}
          animate={{ opacity: 1, y: 0, scale: 1 }}
          exit={{ opacity: 0, y: -8, scale: 0.98 }}
          transition={{ duration: 0.18, ease: "easeOut" }}
          className="absolute top-[calc(100%-12px)] left-1/2 z-30 w-[260px] -translate-x-1/2 pointer-events-auto"
          onMouseDown={(event) => event.stopPropagation()}
          onClick={(event) => event.stopPropagation()}
        >
          <form
            onSubmit={(event) => void handleSubmit(event)}
            className={cn(
              "rounded-[20px] border p-1.5 shadow-xl backdrop-blur-2xl transition-all duration-300",
              isDark ? "bg-black/80 border-white/10 focus-within:border-white/30" : "bg-white/80 border-black/5 focus-within:border-black/20"
            )}
          >
            <div className="flex items-center gap-1.5">
              <Input
                ref={inputRef}
                type="text"
                value={input}
                onChange={(event) => setInput(event.target.value)}
                placeholder={isTyping ? t("chat.thinking") : t("sprite.askPeekoo")}
                disabled={isTyping}
                className={cn(
                  "h-7 rounded-full px-2.5 text-[11px] text-text-primary placeholder:text-text-muted",
                  "border border-transparent shadow-none focus-visible:ring-0 focus-visible:ring-offset-0",
                  isDark ? "bg-white/5 focus-visible:bg-white/10 focus-visible:border-white/10" : "bg-black/5 focus-visible:bg-black/10 focus-visible:border-black/10"
                )}
              />
              <Button
                type="submit"
                size="icon"
                variant="gradient"
                disabled={isTyping || !input.trim()}
                className="h-7 w-7 rounded-full"
              >
                <Send size={12} />
              </Button>
              <button
                type="button"
                onClick={() => void onOpenFullChat()}
                aria-label={t("sprite.openFullChat")}
                title={t("sprite.openFullChat")}
                className={cn(
                  "inline-flex h-7 w-7 items-center justify-center rounded-full transition-colors hover:text-glow-cyan flex-shrink-0",
                  isDark ? "text-white/40 hover:bg-white/5" : "text-black/40 hover:bg-black/5"
                )}
              >
                <Expand size={11} />
              </button>
            </div>
          </form>
        </motion.div>
      ) : null}
    </AnimatePresence>
  );
}
