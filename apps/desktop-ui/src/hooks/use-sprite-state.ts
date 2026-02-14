import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { SpriteState } from "@/types/sprite";

export function useSpriteState() {
  const [spriteState, setSpriteState] = useState<SpriteState>({
    mood: "happy",
    message: "Welcome to Peekoo! Your AI desktop sprite is ready to help you!",
    animation: "happy",
  });

  useEffect(() => {
    invoke<SpriteState>("get_sprite_state")
      .then((state) => {
        setSpriteState(state);
      })
      .catch((error) => {
        console.error("Failed to fetch sprite state:", error);
      });
  }, []);

  return spriteState;
}
