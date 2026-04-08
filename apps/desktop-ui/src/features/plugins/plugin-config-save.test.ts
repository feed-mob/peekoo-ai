import { describe, expect, test } from "bun:test";
import type { PluginConfigField } from "@/types/plugin";
import { buildPluginSaveRequests } from "./plugin-config-save";

const fields: PluginConfigField[] = [
  {
    pluginKey: "health-reminders",
    key: "water_interval_min",
    label: "Water",
    description: null,
    type: "integer",
    default: 45,
    min: 5,
    max: 180,
    options: null,
  },
  {
    pluginKey: "health-reminders",
    key: "eye_rest_interval_min",
    label: "Eyes",
    description: null,
    type: "integer",
    default: 20,
    min: 5,
    max: 120,
    options: null,
  },
];

describe("buildPluginSaveRequests", () => {
  test("routes health reminder settings through health_configure", () => {
    const requests = buildPluginSaveRequests("health-reminders", fields, {
      water_interval_min: 60,
      eye_rest_interval_min: 25,
    });

    expect(requests).toEqual([
      {
        command: "plugin_call_tool",
        payload: {
          toolName: "health_configure",
          argsJson: JSON.stringify({
            water_interval_min: 60,
            eye_rest_interval_min: 25,
          }),
        },
      },
    ]);
  });

  test("keeps generic plugin settings on plugin_config_set", () => {
    const requests = buildPluginSaveRequests("demo-plugin", fields, {
      water_interval_min: 60,
      eye_rest_interval_min: 25,
    });

    expect(requests).toEqual([
      {
        command: "plugin_config_set",
        payload: {
          pluginKey: "demo-plugin",
          key: "water_interval_min",
          value: 60,
        },
      },
      {
        command: "plugin_config_set",
        payload: {
          pluginKey: "demo-plugin",
          key: "eye_rest_interval_min",
          value: 25,
        },
      },
    ]);
  });
});
