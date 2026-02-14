import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { PetReactionEventSchema } from "@/types/pet-event";

export function useSpriteReactions() {
  useEffect(() => {
    const unlisten = listen("pet:react", (event) => {
      const parsed = PetReactionEventSchema.safeParse(event.payload);
      if (!parsed.success) return;

      // TODO: Map trigger to sprite animation change
      // This will integrate with useSpriteState to change mood/animation
      console.log("Pet reaction:", parsed.data.trigger);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);
}
