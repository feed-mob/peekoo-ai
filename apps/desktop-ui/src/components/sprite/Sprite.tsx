import { useState, useEffect } from "react";
import SpriteAnimation from "./SpriteAnimation";
import type { AnimationType, SpriteState, SpriteManifest } from "@/types/sprite";

// Map mood states to sprite animation types (new sprite sheet layout)
const MOOD_TO_ANIMATION: Record<string, AnimationType> = {
  happy: "happy",
  sad: "sleepy",        // Sleepy/rest is the closest to sad/down states
  working: "working",   // Focus state used during active pomodoro sessions
  thinking: "thinking", // Now has a dedicated animation row
  idle: "idle",
  tired: "sleepy",
  reminder: "reminder", // Direct mapping to reminder animation row
};


// Map animation names from backend to animation types (backward compatibility)
const ANIMATION_TO_TYPE: Record<string, AnimationType> = {
  bounce: "happy",
  bounceFast: "happy",
  pulse: "idle",
  pulseFast: "happy",
  shake: "reminder",
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
  animationOverride?: AnimationType | null;
}

export function Sprite({ state, randomTrigger = 0, animationOverride = null }: SpriteProps) {
  const spriteState: SpriteState = state || {
    mood: "happy",
    message: "Welcome! Your AI desktop sprite is ready to help you!",
    animation: "bounce",
  };

  const [overrideAnimation, setOverrideAnimation] = useState<AnimationType | null>(null);
  const [activeSpriteId] = useState("dark-cat");
  const [manifest, setManifest] = useState<SpriteManifest | null>(null);

  // Load sprite manifest
  useEffect(() => {
    fetch(`/sprites/${activeSpriteId}/manifest.json`)
      .then((res) => res.json())
      .then((data: SpriteManifest) => setManifest(data))
      .catch((err) => console.error("Failed to load sprite manifest", err));
  }, [activeSpriteId]);

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
    // External override (e.g. dragging) takes highest priority
    if (animationOverride) {
      return animationOverride;
    }
    // If internal override is active (e.g. random click animation), use it
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

  if (!manifest) {
    return null;
  }

  return (
    <div className="flex items-center justify-center">
      <SpriteAnimation
        animation={getAnimationType()}
        frameRate={manifest.frameRate || 8}
        scale={manifest.scale ?? 0.40}
        chromaKey={manifest.chromaKey}
        imageSrc={`/sprites/${activeSpriteId}/${manifest.image}`}
        columns={manifest.layout.columns}
        rows={manifest.layout.rows}
        pixelArt={manifest.chromaKey.pixelArt}
        onFrameChange={() => {
          // Optional: Log frame changes for debugging
        }}
      />
    </div>
  );
}
