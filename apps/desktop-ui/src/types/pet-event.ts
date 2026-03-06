import { z } from "zod";

export const PetReactionTriggerSchema = z.enum([
  "chat-message",
  "ai-processing",    // AI is actively thinking/processing a response
  "agent-result",
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
  sticky: z.boolean().optional(),
});
export type PetReactionEvent = z.infer<typeof PetReactionEventSchema>;
