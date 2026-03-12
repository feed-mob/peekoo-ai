import { useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { PetReactionEventSchema, type PetReactionTrigger } from "@/types/pet-event";

// Map reaction triggers to mood states aligned with the new sprite sheet rows
const TRIGGER_TO_MOOD: Record<PetReactionTrigger, string> = {
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
    (trigger: PetReactionTrigger, sticky: boolean) => {
      const mood = TRIGGER_TO_MOOD[trigger];
      if (mood) {
        onMoodChange?.(mood, sticky);
      }
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
