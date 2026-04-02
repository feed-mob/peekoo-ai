import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import type { SpriteInfo } from "@/types/global-settings";

interface GlobalSettingsState {
  activeSpriteId: string | null;
  themeMode: string | null;
  logLevel: string | null;
  sprites: SpriteInfo[];
  loading: boolean;
  error: string | null;
}

export function useGlobalSettings() {
  const [state, setState] = useState<GlobalSettingsState>({
    activeSpriteId: null,
    themeMode: null,
    logLevel: null,
    sprites: [],
    loading: true,
    error: null,
  });

  const load = useCallback(async () => {
    try {
      const [settings, sprites] = await Promise.all([
        invoke<Record<string, string>>("app_settings_get"),
        invoke<SpriteInfo[]>("app_settings_list_sprites"),
      ]);
      setState({
        activeSpriteId: settings.active_sprite_id ?? "dark-cat",
        themeMode: settings.theme_mode ?? "system",
        logLevel: settings.log_level ?? "info",
        sprites,
        loading: false,
        error: null,
      });
    } catch (err) {
      setState((prev) => ({
        ...prev,
        loading: false,
        error: String(err),
      }));
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const setActiveSpriteId = useCallback(
    async (spriteId: string) => {
      try {
        await invoke("app_settings_set", {
          key: "active_sprite_id",
          value: spriteId,
        });
        setState((prev) => ({ ...prev, activeSpriteId: spriteId }));
        await emit("sprite:changed", { id: spriteId });
      } catch (err) {
        setState((prev) => ({ ...prev, error: String(err) }));
      }
    },
    [],
  );

  const setThemeMode = useCallback(
    async (mode: string) => {
      try {
        await invoke("app_settings_set", {
          key: "theme_mode",
          value: mode,
        });
        setState((prev) => ({ ...prev, themeMode: mode }));
        await emit("theme:changed", { mode });
      } catch (err) {
        setState((prev) => ({ ...prev, error: String(err) }));
      }
    },
    [],
  );

  const setLogLevel = useCallback(
    async (level: string) => {
      try {
        await invoke("app_settings_set", {
          key: "log_level",
          value: level,
        });
        setState((prev) => ({ ...prev, logLevel: level }));
      } catch (err) {
        setState((prev) => ({ ...prev, error: String(err) }));
      }
    },
    [],
  );

  return {
    activeSpriteId: state.activeSpriteId,
    themeMode: state.themeMode,
    logLevel: state.logLevel,
    sprites: state.sprites,
    loading: state.loading,
    error: state.error,
    setActiveSpriteId,
    setThemeMode,
    setLogLevel,
  };
}
