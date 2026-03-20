import { describe, expect, test } from "bun:test";
import { injectPluginPanelBridge, injectPluginPanelTheme } from "../src/lib/plugin-panel-bridge";

describe("injectPluginPanelBridge", () => {
  test("injects the bridge before panel scripts run", () => {
    const html = `<!doctype html><html><head><title>Plugin</title></head><body><main>Hi</main><script>window.__TAURI__.core.invoke("plugin_call_tool")</script></body></html>`;

    const result = injectPluginPanelBridge(html);

    expect(result).toContain("window.__TAURI__ = window.__TAURI__ || {};");
    expect(result.indexOf("window.__TAURI__ = window.__TAURI__ || {};")).toBeLessThan(
      result.indexOf('window.__TAURI__.core.invoke("plugin_call_tool")'),
    );
  });

  test("injects host theme variables before the panel styles", () => {
    const html = `<!doctype html><html><head><title>Plugin</title></head><body><main>Hi</main></body></html>`;

    const result = injectPluginPanelTheme(html, {
      "--glass": "rgba(10, 20, 30, 0.6)",
      "--glass-border": "rgba(255, 255, 255, 0.15)",
      "--text-primary": "rgb(253, 247, 244)",
    });

    expect(result).toContain(":root {");
    expect(result).toContain("--glass: rgba(10, 20, 30, 0.6);");
    expect(result.indexOf(":root {")).toBeLessThan(result.indexOf("</head>"));
  });
});
