export {
  customProviderRequestSchema as customRuntimeRequestSchema,
  installRuntimeRequestSchema,
  installRuntimeResponseSchema,
  installationMethodSchema,
  prerequisitesCheckSchema,
  providerConfigSchema as runtimeConfigSchema,
  providerStatusSchema as runtimeStatusSchema,
  runtimeInfoSchema,
  testConnectionResultSchema,
} from "./agent-provider";

export type {
  CustomProviderRequest as CustomRuntimeRequest,
  InstallRuntimeRequest,
  InstallRuntimeResponse,
  InstallationMethod,
  PrerequisitesCheck,
  RuntimeConfig,
  RuntimeInfo,
  RuntimeStatus,
  TestConnectionResult,
} from "./agent-provider";

import { z } from "zod";

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
