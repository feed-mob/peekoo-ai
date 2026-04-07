import { z } from "zod";

// Legacy types - will be removed in Phase 7
// These are kept for backward compatibility during the transition

export const runtimeLlmProviderInfoSchema = z.object({
  id: z.string(),
  runtimeId: z.string(),
  providerId: z.string(),
  displayName: z.string().nullish(),
  apiType: z.string(),
  baseUrl: z.string().nullish(),
  config: z.record(z.string(), z.string()),
  isEnabled: z.boolean(),
  isDefault: z.boolean(),
});

export const runtimeModelInfoSchema = z.object({
  id: z.string(),
  runtimeId: z.string(),
  providerId: z.string().nullish(),
  modelId: z.string(),
  displayName: z.string().nullish(),
  isEnabled: z.boolean(),
  isDefault: z.boolean(),
});

export const runtimeLlmProviderUpsertSchema = z.object({
  providerId: z.string(),
  displayName: z.string().nullish(),
  apiType: z.string(),
  baseUrl: z.string().nullish(),
  config: z.record(z.string(), z.string()),
  isEnabled: z.boolean(),
  isDefault: z.boolean(),
});

export const runtimeModelUpsertSchema = z.object({
  providerId: z.string().nullish(),
  modelId: z.string(),
  displayName: z.string().nullish(),
  isEnabled: z.boolean(),
  isDefault: z.boolean(),
});

export type RuntimeLlmProviderInfo = z.infer<typeof runtimeLlmProviderInfoSchema>;
export type RuntimeModelInfo = z.infer<typeof runtimeModelInfoSchema>;
export type RuntimeLlmProviderUpsert = z.infer<typeof runtimeLlmProviderUpsertSchema>;
export type RuntimeModelUpsert = z.infer<typeof runtimeModelUpsertSchema>;
