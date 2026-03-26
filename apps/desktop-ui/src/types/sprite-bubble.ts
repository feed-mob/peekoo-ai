import { z } from "zod";

export const SPRITE_BUBBLE_EVENT = "sprite:bubble" as const;

export const SpriteBubblePayloadSchema = z.object({
  sourcePlugin: z.string().min(1).optional(),
  panelLabel: z.string().startsWith("panel-").optional(),
  title: z.string().min(1),
  body: z.string().min(1),
  actionUrl: z.string().url().optional(),
  actionLabel: z.string().min(1).optional(),
});

export type SpriteBubblePayload = z.infer<typeof SpriteBubblePayloadSchema>;
