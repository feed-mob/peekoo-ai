import { z } from "zod";

export const PetReactionTriggerSchema = z.enum([
  "chat-message",
  "ai-processing",    // AI is actively thinking/processing a response
  "agent-result",
  "task-completed",
  "pomodoro-started",
  "pomodoro-resting",
  "pomodoro-break",
  "pomodoro-completed",
  "panel-opened",
  "panel-closed",
  "opencode-working",   // OpenCode LLM is actively producing output
  "opencode-done",      // OpenCode agent has answered the question
  "opencode-idle",      // No active OpenCode session
]);
export type PetReactionTrigger = z.infer<typeof PetReactionTriggerSchema>;

export const PetReactionEventSchema = z.object({
  trigger: PetReactionTriggerSchema,
  sticky: z.boolean().optional(),
});
export type PetReactionEvent = z.infer<typeof PetReactionEventSchema>;
