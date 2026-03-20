import { useEffect, useRef, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { Expand, Send } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

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
          className="absolute bottom-2 left-1/2 z-30 w-[calc(100%-16px)] -translate-x-1/2 pointer-events-auto"
          onMouseDown={(event) => event.stopPropagation()}
          onClick={(event) => event.stopPropagation()}
        >
          <form
            onSubmit={(event) => void handleSubmit(event)}
            className="rounded-[20px] border border-glass-border bg-glass/95 p-1.5 shadow-panel backdrop-blur-xl"
          >
            <div className="flex items-center gap-1.5">
              <Input
                ref={inputRef}
                type="text"
                value={input}
                onChange={(event) => setInput(event.target.value)}
                placeholder={isTyping ? "Thinking..." : "Ask Peekoo..."}
                disabled={isTyping}
                className="h-7 rounded-full border-glass-border bg-space-deep/80 px-2.5 text-[11px] text-text-primary placeholder:text-text-muted"
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
                aria-label="Open full chat"
                title="Open full chat"
                className="inline-flex h-7 w-7 items-center justify-center rounded-full border border-glass-border/70 bg-space-deep/50 text-text-secondary transition-colors hover:border-glow-cyan/40 hover:text-glow-cyan"
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
