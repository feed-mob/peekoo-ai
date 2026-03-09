import { AnimatePresence, motion } from "framer-motion";
import { BUBBLE_EXTRA_HEIGHT, BUBBLE_WIDTH, SPRITE_WIDTH } from "@/lib/sprite-bubble-layout";
import type { SpriteBubblePayload } from "@/types/sprite-bubble";

const BUBBLE_LEFT = (SPRITE_WIDTH - BUBBLE_WIDTH) / 2;

interface SpriteBubbleProps {
  payload: SpriteBubblePayload | null;
  visible: boolean;
}

export function SpriteBubble({ payload, visible }: SpriteBubbleProps) {
  return (
    <AnimatePresence>
      {payload && visible ? (
        <motion.div
          key={`${payload.title}-${payload.body}`}
          initial={{ opacity: 0, y: 8, scale: 0.95 }}
          animate={{ opacity: 1, y: 0, scale: 1 }}
          exit={{ opacity: 0, y: 4, scale: 0.97 }}
          transition={{ duration: 0.2, ease: "easeOut" }}
          className="pointer-events-none absolute z-20 rounded-2xl border border-glass-border bg-glass/95 px-4 py-3 shadow-panel backdrop-blur-xl"
          style={{
            top: 8,
            left: BUBBLE_LEFT,
            width: BUBBLE_WIDTH,
            maxHeight: BUBBLE_EXTRA_HEIGHT - 16,
          }}
        >
          {/* Tail pointing down toward the sprite */}
          <div className="absolute bottom-[-7px] left-1/2 h-3.5 w-3.5 -translate-x-1/2 rotate-45 border-b border-r border-glass-border bg-glass/95" />

          <p className="text-[9px] font-semibold uppercase tracking-[0.2em] text-glow-cyan/80">
            {payload.title}
          </p>
          <p className="mt-1 text-[13px] leading-[18px] text-text-primary line-clamp-3">
            {payload.body}
          </p>
        </motion.div>
      ) : null}
    </AnimatePresence>
  );
}
