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

// Map mood states to sprite animation types
const MOOD_TO_ANIMATION: Record<string, AnimationType> = {
  happy: "happy",
  sad: "angry",      // Using angry for sad/emotional states
  excited: "excited",
  thinking: "working",
  idle: "idle",
  tired: "sleepy",
  surprised: "angry", // Using angry row for surprised
};

// Map animation names from backend to animation types
const ANIMATION_TO_TYPE: Record<string, AnimationType> = {
  bounce: "happy",
  bounceFast: "excited",
  pulse: "idle",
  pulseFast: "excited",
  shake: "angry",
  sway: "happy",
  idle: "idle",
};

// Random animation types (rows 1-5): happy, excited, sleepy, working, angry
const RANDOM_ANIMATIONS: AnimationType[] = ["happy", "excited", "sleepy", "working", "angry"];

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
