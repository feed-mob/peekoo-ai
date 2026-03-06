import { emit } from "@tauri-apps/api/event";
import type { PetReactionEvent, PetReactionTrigger } from "@/types/pet-event";

interface EmitPetReactionOptions {
  sticky?: boolean;
}

export async function emitPetReaction(
  trigger: PetReactionTrigger,
  options: EmitPetReactionOptions = {},
): Promise<void> {
  const payload: PetReactionEvent =
    options.sticky === undefined
      ? { trigger }
      : { trigger, sticky: options.sticky };

  try {
    await emit("pet:react", payload);
  } catch (error) {
    console.error("Failed to emit pet reaction:", error);
  }
}
