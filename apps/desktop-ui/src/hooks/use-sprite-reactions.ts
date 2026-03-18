import { useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { PetReactionEventSchema } from "@/types/pet-event";

// Maps app-level semantic triggers to mood states.
// Plugin-emitted values (e.g. "working", "idle") are not listed here —
// they pass through as-is via the fallback in handleReaction.
const TRIGGER_TO_MOOD: Partial<Record<string, string>> = {
  "chat-message": "thinking",      // AI received a message, entering thinking state
  "ai-processing": "thinking",     // AI is actively processing/generating a response
  "agent-result": "reminder",      // AI finished and produced a result
  "task-completed": "happy",       // Celebrate task completion
  "pomodoro-started": "working",   // Enter focus/working mode
  "pomodoro-resting": "sleepy",    // Actively resting during break
  "pomodoro-break": "idle",        // Resting between sessions (paused or not started)
  "pomodoro-completed": "happy",   // Celebrate completing a pomodoro session
  "panel-opened": "reminder",      // Something is being shown to the user
  "panel-closed": "idle",          // Return to neutral idle state
};

interface UseSpriteReactionsOptions {
  onMoodChange?: (mood: string, sticky: boolean) => void;
}

export function useSpriteReactions({ onMoodChange }: UseSpriteReactionsOptions = {}) {
  const handleReaction = useCallback(
    (trigger: string, sticky: boolean) => {
      const mood = TRIGGER_TO_MOOD[trigger] ?? trigger;
      onMoodChange?.(mood, sticky);
    },
    [onMoodChange],
  );

  useEffect(() => {
    const unlisten = listen("pet:react", (event) => {
      const parsed = PetReactionEventSchema.safeParse(event.payload);
      if (!parsed.success) return;

      handleReaction(parsed.data.trigger, parsed.data.sticky ?? false);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [handleReaction]);
}
