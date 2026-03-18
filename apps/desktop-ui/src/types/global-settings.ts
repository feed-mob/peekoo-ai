import { z } from "zod";

export const SpriteInfoSchema = z.object({
  id: z.string(),
  name: z.string(),
  description: z.string(),
});
export type SpriteInfo = z.infer<typeof SpriteInfoSchema>;

export const GlobalSettingsSchema = z.record(z.string(), z.string());
export type GlobalSettings = z.infer<typeof GlobalSettingsSchema>;
