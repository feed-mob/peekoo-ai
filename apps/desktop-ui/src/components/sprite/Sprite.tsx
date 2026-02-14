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

interface SpriteProps {
  state?: SpriteState;
}

export function Sprite({ state }: SpriteProps) {
  const spriteState: SpriteState = state || {
    mood: "happy",
    message: "Welcome! Your AI desktop sprite is ready to help you!",
    animation: "bounce",
  };

  // Determine animation type from mood or animation state
  const getAnimationType = (): AnimationType => {
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
        scale={0.3}
        chromaKey={SPRITE_CHROMA_KEY}
        onFrameChange={() => {
          // Optional: Log frame changes for debugging
        }}
      />
    </div>
  );
}
