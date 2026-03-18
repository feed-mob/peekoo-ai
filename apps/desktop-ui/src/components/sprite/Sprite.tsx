import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
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
  sleepy: "sleepy",     // Directly map sleepy mood to sleepy animation
  reminder: "reminder", // Direct mapping to reminder animation row
  dragging: "dragging",
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
  dragging: "dragging",
};

const DEFAULT_SPRITE_ID = "dark-cat";

interface SpriteProps {
  state?: SpriteState;
  animationOverride?: AnimationType | null;
}

export function Sprite({ state, animationOverride = null }: SpriteProps) {
  const spriteState: SpriteState = state || {
    mood: "happy",
    message: "Welcome! Your AI desktop sprite is ready to help you!",
    animation: "bounce",
  };

  const [activeSpriteId, setActiveSpriteId] = useState(DEFAULT_SPRITE_ID);
  const [manifest, setManifest] = useState<SpriteManifest | null>(null);

  // Load active sprite ID from global settings on mount
  useEffect(() => {
    invoke<Record<string, string>>("app_settings_get")
      .then((settings) => {
        if (settings.active_sprite_id) {
          setActiveSpriteId(settings.active_sprite_id);
        }
      })
      .catch((err) => console.error("Failed to load active sprite setting", err));
  }, []);

  // Listen for sprite changes from the settings panel
  useEffect(() => {
    const unlisten = listen<{ id: string }>("sprite:changed", (event) => {
      setActiveSpriteId(event.payload.id);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Load sprite manifest
  useEffect(() => {
    fetch(`/sprites/${activeSpriteId}/manifest.json`)
      .then((res) => res.json())
      .then((data: SpriteManifest) => setManifest(data))
      .catch((err) => console.error("Failed to load sprite manifest", err));
  }, [activeSpriteId]);

  // Determine animation type from mood or animation state
  const getAnimationType = (): AnimationType => {
    // External override (e.g. dragging) takes highest priority
    if (animationOverride) {
      return animationOverride;
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
