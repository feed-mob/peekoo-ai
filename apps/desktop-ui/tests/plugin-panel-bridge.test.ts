import { describe, expect, test } from "bun:test";
import { injectPluginPanelBridge } from "../src/lib/plugin-panel-bridge";

describe("injectPluginPanelBridge", () => {
  test("injects the bridge before panel scripts run", () => {
    const html = `<!doctype html><html><head><title>Plugin</title></head><body><main>Hi</main><script>window.__TAURI__.core.invoke("plugin_call_tool")</script></body></html>`;

    const result = injectPluginPanelBridge(html);

    expect(result).toContain("window.__TAURI__ = window.__TAURI__ || {};");
    expect(result.indexOf("window.__TAURI__ = window.__TAURI__ || {};")).toBeLessThan(
      result.indexOf('window.__TAURI__.core.invoke("plugin_call_tool")'),
    );
  });
});
