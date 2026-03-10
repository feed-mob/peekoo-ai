import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import {
  type PluginPanel,
  pluginPanelSchema,
  type PluginSummary,
  pluginSummarySchema,
} from "@/types/plugin";

export function usePlugins() {
  const [plugins, setPlugins] = useState<PluginSummary[]>([]);
  const [panels, setPanels] = useState<PluginPanel[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [toggling, setToggling] = useState<Set<string>>(new Set());

  const refresh = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const [rawPlugins, rawPanels] = await Promise.all([
        invoke("plugins_list"),
        invoke("plugin_panels_list"),
      ]);
      setPlugins(pluginSummarySchema.array().parse(rawPlugins));
      setPanels(pluginPanelSchema.array().parse(rawPanels));
    } catch (err) {
      setError(String(err));
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();

    const unlisten = listen("plugins-changed", () => {
      void refresh();
    });

    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [refresh]);

  const setPluginEnabled = useCallback(async (pluginKey: string, enabled: boolean) => {
    setToggling((prev) => new Set(prev).add(pluginKey));
    setError(null);
    try {
      await invoke(enabled ? "plugin_enable" : "plugin_disable", { pluginKey });
      setPlugins((prev) =>
        prev.map((plugin) =>
          plugin.pluginKey === pluginKey ? { ...plugin, enabled } : plugin,
        ),
      );
      if (!enabled) {
        setPanels((prev) => prev.filter((panel) => panel.pluginKey !== pluginKey));
      }
    } catch (err) {
      setError(String(err));
      throw err;
    } finally {
      setToggling((prev) => {
        const next = new Set(prev);
        next.delete(pluginKey);
        return next;
      });
    }
  }, []);

  const isToggling = useCallback(
    (pluginKey: string) => toggling.has(pluginKey),
    [toggling],
  );

  return {
    plugins,
    panels,
    isLoading,
    error,
    refresh,
    setPluginEnabled,
    isToggling,
  };
}
