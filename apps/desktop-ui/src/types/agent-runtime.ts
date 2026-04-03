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

// Legacy types for backward compatibility during transition
// These are re-exported from agent-runtime-legacy and will be removed in Phase 8
export type {
  RuntimeLlmProviderInfo,
  RuntimeLlmProviderUpsert,
  RuntimeModelInfo,
  RuntimeModelUpsert,
} from "./agent-runtime-legacy";

import { z } from "zod";

// New ACP inspection types - models discovered from ACP protocol
export const discoveredModelSchema = z.object({
  modelId: z.string(),
  name: z.string(),
  description: z.string().nullish(),
});

export const authMethodSchema = z.object({
  id: z.string(),
  name: z.string(),
  description: z.string().nullish(),
  manualLoginCommand: z.string().nullish(),
});

export const runtimeAuthenticationStatusSchema = z.enum([
  "authenticated",
  "terminal_login_started",
]);

export const runtimeAuthenticationResultSchema = z.object({
  status: runtimeAuthenticationStatusSchema,
  message: z.string(),
});

export const runtimeInspectionResultSchema = z.object({
  runtimeId: z.string(),
  authMethods: z.array(authMethodSchema),
  authRequired: z.boolean(),
  discoveredModels: z.array(discoveredModelSchema),
  currentModelId: z.string().nullish(),
  supportsModelSelection: z.boolean(),
  supportsConfigOptions: z.boolean(),
  error: z.string().nullish(),
});

export type DiscoveredModel = z.infer<typeof discoveredModelSchema>;
export type AuthMethod = z.infer<typeof authMethodSchema>;
export type RuntimeAuthenticationStatus = z.infer<typeof runtimeAuthenticationStatusSchema>;
export type RuntimeAuthenticationResult = z.infer<typeof runtimeAuthenticationResultSchema>;
export type RuntimeInspectionResult = z.infer<typeof runtimeInspectionResultSchema>;
