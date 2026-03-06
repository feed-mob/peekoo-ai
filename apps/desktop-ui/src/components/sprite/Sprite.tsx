import { useState, useEffect } from "react";
import SpriteAnimation from "./SpriteAnimation";
import type { AnimationType, SpriteState } from "@/types/sprite";

const SPRITE_CHROMA_KEY = {
  targetColor: [255, 0, 255] as const,
  minRbOverG: 38,
  threshold: 84,
  softness: 64,
  spillSuppression: {
    enabled: true,
    threshold: 230,
    strength: 0.78,
  },
};

// Map mood states to sprite animation types (new sprite sheet layout)
const MOOD_TO_ANIMATION: Record<string, AnimationType> = {
  happy: "happy",
  sad: "sleepy",        // Sleepy/rest is the closest to sad/down states
  thinking: "thinking", // Now has a dedicated animation row
  idle: "idle",
  tired: "sleepy",
  reminder: "reminder", // Direct mapping to reminder animation row
};

// Map animation names from backend to animation types (backward compatibility)
const ANIMATION_TO_TYPE: Record<string, AnimationType> = {
  bounce: "happy",
  bounceFast: "happy",    // Was "excited", now maps to happy
  pulse: "idle",
  pulseFast: "happy",     // Was "excited", now maps to happy
  shake: "reminder",      // Was "angry", maps to reminder as a neutral alert
  sway: "happy",
  idle: "idle",
  working: "working",
  thinking: "thinking",
  reminder: "reminder",
};

// Random animation types (rows 1-6, excluding idle and dragging)
const RANDOM_ANIMATIONS: AnimationType[] = ["happy", "working", "thinking", "reminder", "sleepy"];

interface SpriteProps {
  state?: SpriteState;
  randomTrigger?: number;
}

export function Sprite({ state, randomTrigger = 0 }: SpriteProps) {
  const spriteState: SpriteState = state || {
    mood: "happy",
    message: "Welcome! Your AI desktop sprite is ready to help you!",
    animation: "bounce",
  };

  const [overrideAnimation, setOverrideAnimation] = useState<AnimationType | null>(null);

  // Trigger random animation when randomTrigger changes
  useEffect(() => {
    if (randomTrigger > 0) {
      const randomIndex = Math.floor(Math.random() * RANDOM_ANIMATIONS.length);
      const randomAnim = RANDOM_ANIMATIONS[randomIndex];
      setOverrideAnimation(randomAnim);

      // Reset after 2 seconds (animation plays for a bit then reverts)
      const timeout = setTimeout(() => {
        setOverrideAnimation(null);
      }, 2000);

      return () => clearTimeout(timeout);
    }
  }, [randomTrigger]);

  // Determine animation type from mood or animation state
  const getAnimationType = (): AnimationType => {
    // If override is active, use it
    if (overrideAnimation) {
      return overrideAnimation;
    }

    // First try to map from mood
    if (MOOD_TO_ANIMATION[spriteState.mood]) {
      return MOOD_TO_ANIMATION[spriteState.mood];
    }
    // Then try to map from animation string
    if (ANIMATION_TO_TYPE[spriteState.animation]) {
      return ANIMATION_TO_TYPE[spriteState.animation];
    }
    // Default to idle
    return "idle";
  };

  return (
    <div className="flex items-center justify-center">
      <SpriteAnimation
        animation={getAnimationType()}
        frameRate={8}
        scale={0.2}
        chromaKey={SPRITE_CHROMA_KEY}
        onFrameChange={() => {
          // Optional: Log frame changes for debugging
        }}
      />
    </div>
  );
}
