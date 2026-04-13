import { useState, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import SpriteAnimation from "./SpriteAnimation";
import { getActiveSpriteManifest } from "./spriteManifest";
import { loadSpriteAsset } from "./spriteAsset";
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
  const [sprites, setSprites] = useState<Record<string, SpriteInfo>>({});
  const [manifests, setManifests] = useState<Record<string, SpriteManifest>>({});
  const [imageSources, setImageSources] = useState<Record<string, string>>({});
  const [spriteVisible, setSpriteVisible] = useState(true);

  // Load active sprite ID from global settings on mount
  useEffect(() => {
    Promise.all([
      invoke<Record<string, string>>("app_settings_get"),
      invoke<SpriteInfo[]>("app_settings_list_sprites"),
    ])
      .then(([settings, availableSprites]) => {
        setSprites(Object.fromEntries(availableSprites.map((sprite) => [sprite.id, sprite])));
        if (settings.active_sprite_id) {
          setActiveSpriteId(settings.active_sprite_id);
        }
      })
      .catch((err) => console.error("Failed to load sprite settings", err));
  }, []);

  // Listen for sprite changes from the settings panel
  useEffect(() => {
    const unlisten = listen<{ id: string }>("sprite:changed", (event) => {
      setActiveSpriteId(event.payload.id);
      void invoke<SpriteInfo[]>("app_settings_list_sprites")
        .then((availableSprites) => {
          setSprites(Object.fromEntries(availableSprites.map((sprite) => [sprite.id, sprite])));
        })
        .catch((err) => console.error("Failed to refresh sprite catalog", err));
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Load sprite assets
  useEffect(() => {
    let cancelled = false;
    const availableSprites = Object.values(sprites);
    if (availableSprites.length === 0) {
      return;
    }

    const loadAssets = async () => {
      const loadedAssets = await Promise.all(
        availableSprites.map(async (sprite) => {
          try {
            const asset = await loadSpriteAsset(sprite);
            return [sprite.id, asset] as const;
          } catch (err) {
            console.error(`Failed to load sprite asset for ${sprite.id}`, err);
            return null;
          }
        }),
      );

      if (cancelled) {
        return;
      }

      const nextManifests: Record<string, SpriteManifest> = {};
      const nextImageSources: Record<string, string> = {};
      for (const loaded of loadedAssets) {
        if (!loaded) {
          continue;
        }
        nextManifests[loaded[0]] = loaded[1].manifest;
        nextImageSources[loaded[0]] = loaded[1].imageSrc;
      }
      setManifests((prev) => ({ ...prev, ...nextManifests }));
      setImageSources((prev) => ({ ...prev, ...nextImageSources }));
    };

    void loadAssets();

    return () => {
      cancelled = true;
    };
  }, [sprites]);

  const activeManifest = getActiveSpriteManifest(manifests, activeSpriteId);
  const activeImageSrc = imageSources[activeSpriteId];

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

  if (!activeManifest || !activeImageSrc) {
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
        imageSrc={activeImageSrc}
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
