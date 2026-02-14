import { z } from "zod";

export const PetReactionTriggerSchema = z.enum([
  "chat-message",
  "task-completed",
  "pomodoro-started",
  "pomodoro-break",
  "pomodoro-completed",
  "panel-opened",
  "panel-closed",
]);
export type PetReactionTrigger = z.infer<typeof PetReactionTriggerSchema>;

export const PetReactionEventSchema = z.object({
  trigger: PetReactionTriggerSchema,
});
export type PetReactionEvent = z.infer<typeof PetReactionEventSchema>;
