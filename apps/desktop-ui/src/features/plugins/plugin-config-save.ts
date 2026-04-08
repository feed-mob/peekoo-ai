import type { PluginConfigField } from "@/types/plugin";

type ConfigValues = Record<string, unknown>;

export interface PluginSaveRequest {
  command: string;
  payload: Record<string, unknown>;
}

export function buildPluginSaveRequests(
  pluginKey: string,
  fields: PluginConfigField[],
  values: ConfigValues,
): PluginSaveRequest[] {
  if (pluginKey === "health-reminders") {
    const patch = Object.fromEntries(
      fields.map((field) => [field.key, values[field.key] ?? field.default]),
    );

    return [
      {
        command: "plugin_call_tool",
        payload: {
          toolName: "health_configure",
          argsJson: JSON.stringify(patch),
        },
      },
    ];
  }

  return fields.map((field) => ({
    command: "plugin_config_set",
    payload: {
      pluginKey,
      key: field.key,
      value: values[field.key] ?? field.default,
    },
  }));
}
