import { z } from "zod";

export const SPRITE_BUBBLE_EVENT = "sprite:bubble" as const;
export const SPRITE_BUBBLE_DURATION_MS = 5000;

export const SpriteBubblePayloadSchema = z.object({
  title: z.string().min(1),
  body: z.string().min(1),
  actionUrl: z.string().url().optional(),
  actionLabel: z.string().min(1).optional(),
});

export type SpriteBubblePayload = z.infer<typeof SpriteBubblePayloadSchema>;
