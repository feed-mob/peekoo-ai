import { z } from "zod";

export const PEEK_BADGES_EVENT = "sprite:peek-badges" as const;

export const PeekBadgeItemSchema = z.object({
  label: z.string().min(1),
  value: z.string().min(1),
  icon: z.string().optional(),
  target_epoch_secs: z.number().optional(),
});

export const PeekBadgesPayloadSchema = z.array(PeekBadgeItemSchema);

export type PeekBadgeItem = z.infer<typeof PeekBadgeItemSchema>;
