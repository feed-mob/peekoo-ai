import { useCallback, useRef, useState } from "react";
import { SPRITE_BUBBLE_DURATION_MS, type SpriteBubblePayload } from "@/types/sprite-bubble";

const FADE_OUT_DURATION_MS = 220;

export function useSpriteBubble() {
  const [payload, setPayload] = useState<SpriteBubblePayload | null>(null);
  const [visible, setVisible] = useState(false);
  const hideTimerRef = useRef<number | null>(null);
  const clearTimerRef = useRef<number | null>(null);

  const clearTimers = useCallback(() => {
    if (hideTimerRef.current !== null) {
      window.clearTimeout(hideTimerRef.current);
      hideTimerRef.current = null;
    }

    if (clearTimerRef.current !== null) {
      window.clearTimeout(clearTimerRef.current);
      clearTimerRef.current = null;
    }
  }, []);

  const showBubble = useCallback((nextPayload: SpriteBubblePayload) => {
    clearTimers();
    setPayload(nextPayload);
    setVisible(true);

    hideTimerRef.current = window.setTimeout(() => {
      setVisible(false);
      clearTimerRef.current = window.setTimeout(() => {
        setPayload(null);
        clearTimerRef.current = null;
      }, FADE_OUT_DURATION_MS);
    }, SPRITE_BUBBLE_DURATION_MS);
  }, [clearTimers]);

  const clearBubble = useCallback(() => {
    clearTimers();
    setVisible(false);
    setPayload(null);
  }, [clearTimers]);

  return {
    payload,
    visible,
    showBubble,
    clearBubble,
  };
}
