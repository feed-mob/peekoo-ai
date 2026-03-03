import { z } from "zod";

export const providerAuthSchema = z.object({
  providerId: z.string(),
  authMode: z.string(),
  configured: z.boolean(),
  oauthExpiresAt: z.string().nullable().optional(),
});

export const skillSchema = z.object({
  skillId: z.string(),
  sourceType: z.string(),
  path: z.string(),
  enabled: z.boolean(),
});

export const agentSettingsSchema = z.object({
  activeProviderId: z.string(),
  activeModelId: z.string(),
  systemPrompt: z.string().nullable().optional(),
  maxToolIterations: z.number(),
  version: z.number(),
  providerAuth: z.array(providerAuthSchema),
  skills: z.array(skillSchema),
});

export const providerCatalogSchema = z.object({
  id: z.string(),
  name: z.string(),
  authModes: z.array(z.string()),
  models: z.array(z.string()),
});

export const agentSettingsCatalogSchema = z.object({
  providers: z.array(providerCatalogSchema),
  discoveredSkills: z.array(skillSchema),
});

export type ProviderAuth = z.infer<typeof providerAuthSchema>;
export type SkillSettings = z.infer<typeof skillSchema>;
export type AgentSettings = z.infer<typeof agentSettingsSchema>;
export type ProviderCatalog = z.infer<typeof providerCatalogSchema>;
export type AgentSettingsCatalog = z.infer<typeof agentSettingsCatalogSchema>;
