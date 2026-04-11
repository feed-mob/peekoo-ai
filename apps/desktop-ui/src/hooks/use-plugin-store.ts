import { invoke } from "@tauri-apps/api/core";
import { useCallback, useRef, useState } from "react";
import { storePluginSchema, type StorePlugin } from "@/types/plugin";

export function usePluginStore() {
  const [catalog, setCatalog] = useState<StorePlugin[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [installing, setInstalling] = useState<Set<string>>(new Set());
  const fetchingRef = useRef(false);

  const fetchCatalog = useCallback(async () => {
    if (fetchingRef.current) return;
    fetchingRef.current = true;
    setIsLoading(true);
    setError(null);
    try {
      const rawCatalog = await invoke("plugin_store_catalog");
      setCatalog(storePluginSchema.array().parse(rawCatalog));
    } catch (err) {
      setError(String(err));
    } finally {
      setIsLoading(false);
      fetchingRef.current = false;
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
            ? installedPlugin
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

  const update = useCallback(async (pluginKey: string) => {
    setInstalling((prev) => new Set(prev).add(pluginKey));
    setError(null);
    try {
      const rawPlugin = await invoke("plugin_store_update", { pluginKey });
      const updatedPlugin = storePluginSchema.parse(rawPlugin);
      setCatalog((prev) =>
        prev.map((p) =>
          p.pluginKey === pluginKey
            ? { ...updatedPlugin, hasUpdate: false }
            : p
        )
      );
      return updatedPlugin;
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
            ? { ...p, installed: false, source: "none", hasUpdate: false }
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

  const isInstalling = useCallback(
    (pluginKey: string) => installing.has(pluginKey),
    [installing],
  );

  return {
    catalog,
    isLoading,
    error,
    fetchCatalog,
    install,
    update,
    uninstall,
    isInstalling,
  };
}
