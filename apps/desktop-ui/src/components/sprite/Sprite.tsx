import { useState, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import SpriteAnimation from "./SpriteAnimation";
import { getActiveSpriteManifest } from "./spriteManifest";
import type { SpriteInfo } from "@/types/global-settings";
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
const SPRITE_SWITCH_FADE_MS = 150;

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
  const [manifests, setManifests] = useState<Record<string, SpriteManifest>>({});
  const [spriteVisible, setSpriteVisible] = useState(true);

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
    let cancelled = false;

    const loadManifest = async (spriteId: string) => {
      try {
        const response = await fetch(`/sprites/${spriteId}/manifest.json`);
        const data = (await response.json()) as SpriteManifest;
        if (cancelled) {
          return;
        }
        setManifests((prev) => {
          if (prev[spriteId] === data) {
            return prev;
          }
          return {
            ...prev,
            [spriteId]: data,
          };
        });
      } catch (err) {
        console.error("Failed to load sprite manifest", err);
      }
    };

    void loadManifest(activeSpriteId);

    invoke<SpriteInfo[]>("app_settings_list_sprites")
      .then((sprites) => {
        sprites
          .filter((sprite) => sprite.id !== activeSpriteId)
          .forEach((sprite) => {
            void loadManifest(sprite.id);
          });
      })
      .catch((err) => console.error("Failed to prefetch sprite manifests", err));

    return () => {
      cancelled = true;
    };
  }, [activeSpriteId]);

  const activeManifest = getActiveSpriteManifest(manifests, activeSpriteId);

  useEffect(() => {
    if (!activeManifest) {
      return;
    }

    setSpriteVisible(false);
    const timeoutId = window.setTimeout(() => {
      setSpriteVisible(true);
    }, 0);

    return () => {
      window.clearTimeout(timeoutId);
    };
  }, [activeSpriteId, activeManifest]);

  const spriteClasses = useMemo(
    () =>
      spriteVisible
        ? "opacity-100"
        : "opacity-0",
    [spriteVisible],
  );

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

  if (!activeManifest) {
    return null;
  }

  return (
    <div
      className={`flex items-center justify-center transition-opacity ease-out ${spriteClasses}`}
      style={{ transitionDuration: `${SPRITE_SWITCH_FADE_MS}ms` }}
    >
      <SpriteAnimation
        key={activeSpriteId}
        animation={getAnimationType()}
        frameRate={activeManifest.frameRate || 8}
        scale={activeManifest.scale ?? 0.40}
        chromaKey={activeManifest.chromaKey}
        imageSrc={`/sprites/${activeSpriteId}/${activeManifest.image}`}
        columns={activeManifest.layout.columns}
        rows={activeManifest.layout.rows}
        pixelArt={activeManifest.chromaKey.pixelArt}
        onFrameChange={() => {
          // Optional: Log frame changes for debugging
        }}
      />
    </div>
  );
}
