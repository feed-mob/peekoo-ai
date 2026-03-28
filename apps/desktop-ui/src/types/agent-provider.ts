import { z } from "zod";

// Installation method enum
export const installationMethodSchema = z.enum(["bundled", "npx", "binary", "custom"]);

// Provider status enum
export const providerStatusSchema = z.enum([
  "not_installed",
  "installing",
  "ready",
  "error",
  "needs_setup",
]);

// Provider configuration
export const providerConfigSchema = z.object({
  defaultModel: z.string().optional(),
  envVars: z.record(z.string()).default({}),
  customArgs: z.array(z.string()).default([]),
});

// Installation method info
export const installationMethodInfoSchema = z.object({
  id: installationMethodSchema,
  name: z.string(),
  description: z.string(),
  isAvailable: z.boolean(),
  requiresSetup: z.boolean(),
  sizeMb: z.number().optional(),
});

// Provider info
export const providerInfoSchema = z.object({
  id: z.string(),
  providerId: z.string(),
  displayName: z.string(),
  description: z.string(),
  isBundled: z.boolean(),
  installationMethod: installationMethodSchema,
  isInstalled: z.boolean(),
  isDefault: z.boolean(),
  status: providerStatusSchema,
  statusMessage: z.string().optional(),
  availableMethods: z.array(installationMethodInfoSchema),
  config: providerConfigSchema,
});

// Install request
export const installProviderRequestSchema = z.object({
  providerId: z.string(),
  method: installationMethodSchema,
  customPath: z.string().optional(),
});

// Install response
export const installProviderResponseSchema = z.object({
  success: z.boolean(),
  message: z.string(),
  requiresRestart: z.boolean(),
});

// Test connection result
export const testConnectionResultSchema = z.object({
  success: z.boolean(),
  message: z.string(),
  availableModels: z.array(z.string()),
  providerVersion: z.string().optional(),
});

// Prerequisites check
export const prerequisitesCheckSchema = z.object({
  available: z.boolean(),
  missingComponents: z.array(z.string()),
  instructions: z.string().optional(),
});

// Custom provider request
export const customProviderRequestSchema = z.object({
  name: z.string(),
  description: z.string().optional(),
  command: z.string(),
  args: z.array(z.string()).default([]),
  workingDir: z.string().optional(),
});

// Type exports
export type InstallationMethod = z.infer<typeof installationMethodSchema>;
export type ProviderStatus = z.infer<typeof providerStatusSchema>;
export type ProviderConfig = z.infer<typeof providerConfigSchema>;
export type InstallationMethodInfo = z.infer<typeof installationMethodInfoSchema>;
export type ProviderInfo = z.infer<typeof providerInfoSchema>;
export type InstallProviderRequest = z.infer<typeof installProviderRequestSchema>;
export type InstallProviderResponse = z.infer<typeof installProviderResponseSchema>;
export type TestConnectionResult = z.infer<typeof testConnectionResultSchema>;
export type PrerequisitesCheck = z.infer<typeof prerequisitesCheckSchema>;
export type CustomProviderRequest = z.infer<typeof customProviderRequestSchema>;
