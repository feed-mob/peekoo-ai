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

  return {
    plugins,
    panels,
    isLoading,
    error,
    refresh,
  };
}
