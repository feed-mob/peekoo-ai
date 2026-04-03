import { z } from "zod";

export const installationMethodSchema = z.enum(["npx", "binary", "uvx"]);
export type InstallationMethod = z.infer<typeof installationMethodSchema>;

export const registryAgentSchema = z.object({
  registryId: z.string(),
  name: z.string(),
  version: z.string(),
  description: z.string(),
  authors: z.array(z.string()),
  license: z.string(),
  website: z.string().nullish(),
  iconUrl: z.string().nullish(),
  supportedPlatforms: z.array(z.string()),
  supportedMethods: z.array(z.string()),
  isSupportedOnCurrentPlatform: z.boolean(),
  preferredMethod: z.string().nullish(),
  isInstalled: z.boolean(),
  installedVersion: z.string().nullish(),
  displayOrder: z.number(),
});

export type RegistryAgent = z.infer<typeof registryAgentSchema>;

export const paginatedRegistryAgentsSchema = z.object({
  agents: z.array(registryAgentSchema),
  totalCount: z.number(),
  page: z.number(),
  pageSize: z.number(),
  hasMore: z.boolean(),
});

export type PaginatedRegistryAgents = z.infer<typeof paginatedRegistryAgentsSchema>;
