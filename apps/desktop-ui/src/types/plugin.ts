import { z } from "zod";

export const pluginSummarySchema = z.object({
  pluginKey: z.string(),
  name: z.string(),
  version: z.string(),
  author: z.string().nullable().optional(),
  description: z.string().nullable().optional(),
  enabled: z.boolean(),
  toolCount: z.number(),
  panelCount: z.number(),
  pluginDir: z.string(),
});

export const pluginPanelSchema = z.object({
  pluginKey: z.string(),
  label: z.string(),
  title: z.string(),
  width: z.number(),
  height: z.number(),
  entry: z.string(),
});

export const storePluginSchema = z.object({
  pluginKey: z.string(),
  name: z.string(),
  version: z.string(),
  author: z.string().nullable().optional(),
  description: z.string().nullable().optional(),
  permissions: z.array(z.string()),
  toolCount: z.number(),
  panelCount: z.number(),
  installed: z.boolean(),
  source: z.enum(["store", "none"]),
  hasUpdate: z.boolean(),
});

export const pluginConfigOptionSchema = z.object({
  value: z.string(),
  label: z.string(),
});

export const pluginConfigFieldSchema = z.object({
  pluginKey: z.string(),
  key: z.string(),
  label: z.string(),
  description: z.string().nullable().optional(),
  type: z.enum(["integer", "boolean", "string", "select"]),
  default: z.unknown(),
  min: z.number().nullable().optional(),
  max: z.number().nullable().optional(),
  options: pluginConfigOptionSchema.array().nullable().optional(),
});

export type PluginSummary = z.infer<typeof pluginSummarySchema>;
export type PluginPanel = z.infer<typeof pluginPanelSchema>;
export type StorePlugin = z.infer<typeof storePluginSchema>;
export type PluginConfigField = z.infer<typeof pluginConfigFieldSchema>;
