import { invoke } from "@tauri-apps/api/core";
import { useCallback, useState } from "react";
import { storePluginSchema, type StorePlugin } from "@/types/plugin";

export function usePluginStore() {
  const [catalog, setCatalog] = useState<StorePlugin[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [installing, setInstalling] = useState<Set<string>>(new Set());

  const fetchCatalog = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const rawCatalog = await invoke("plugin_store_catalog");
      setCatalog(storePluginSchema.array().parse(rawCatalog));
    } catch (err) {
      setError(String(err));
    } finally {
      setIsLoading(false);
    }
  }, []);

  const install = useCallback(async (pluginKey: string) => {
    setInstalling((prev) => new Set(prev).add(pluginKey));
    setError(null);
    try {
      const rawPlugin = await invoke("plugin_store_install", { pluginKey });
      const installedPlugin = storePluginSchema.parse(rawPlugin);
      setCatalog((prev) =>
        prev.map((p) =>
          p.pluginKey === pluginKey
            ? { ...p, installed: true, source: "store" }
            : p
        )
      );
      return installedPlugin;
    } catch (err) {
      setError(String(err));
      throw err;
    } finally {
      setInstalling((prev) => {
        const next = new Set(prev);
        next.delete(pluginKey);
        return next;
      });
    }
  }, []);

  const uninstall = useCallback(async (pluginKey: string) => {
    setInstalling((prev) => new Set(prev).add(pluginKey));
    setError(null);
    try {
      await invoke("plugin_store_uninstall", { pluginKey });
      setCatalog((prev) =>
        prev.map((p) =>
          p.pluginKey === pluginKey
            ? { ...p, installed: false, source: "none" }
            : p
        )
      );
    } catch (err) {
      setError(String(err));
      throw err;
    } finally {
      setInstalling((prev) => {
        const next = new Set(prev);
        next.delete(pluginKey);
        return next;
      });
    }
  }, []);

  const isInstalling = (pluginKey: string) => installing.has(pluginKey);

  return {
    catalog,
    isLoading,
    error,
    fetchCatalog,
    install,
    uninstall,
    isInstalling,
  };
}
