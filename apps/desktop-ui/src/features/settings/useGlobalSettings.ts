import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import type { SpriteInfo } from "@/types/global-settings";
import type {
  GenerateSpriteManifestInput,
  GeneratedSpriteManifest,
  SaveCustomSpriteInput,
  SpriteManifest,
  SpriteManifestValidation,
} from "@/types/sprite";

interface GlobalSettingsState {
  activeSpriteId: string | null;
  themeMode: string | null;
  appLanguage: string | null;
  logLevel: string | null;
  sprites: SpriteInfo[];
  loading: boolean;
  error: string | null;
}

export function useGlobalSettings() {
  const [state, setState] = useState<GlobalSettingsState>({
    activeSpriteId: null,
    themeMode: null,
    appLanguage: null,
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
        appLanguage: settings.app_language ?? "en",
        logLevel: settings.log_level ?? "error",
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

  const setAppLanguage = useCallback(
    async (language: string) => {
      try {
        await invoke("app_settings_set_language", { language });
        setState((prev) => ({ ...prev, appLanguage: language }));
        await emit("language:changed", { language });
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

  const getSpritePrompt = useCallback(
    () => invoke<string>("app_sprites_get_prompt"),
    [],
  );

  const getSpriteManifestTemplate = useCallback(
    () => invoke<SpriteManifest>("app_sprites_get_manifest_template"),
    [],
  );

  const loadSpriteManifestFile = useCallback(
    (manifestPath: string) => invoke<SpriteManifest>("app_sprites_load_manifest_file", { manifestPath }),
    [],
  );

  const generateSpriteManifestDraft = useCallback(
    (input: GenerateSpriteManifestInput) =>
      invoke<GeneratedSpriteManifest>("app_sprites_generate_manifest_draft", { input }),
    [],
  );

  const generateSpriteManifestWithAgent = useCallback(
    (input: GenerateSpriteManifestInput) =>
      invoke<GeneratedSpriteManifest>("app_sprites_generate_manifest_with_agent", { input }),
    [],
  );

  const validateSpriteManifest = useCallback(
    (input: { imagePath: string; manifest: SpriteManifest }) =>
      invoke<SpriteManifestValidation>("app_sprites_validate_manifest", { input }),
    [],
  );

  const saveCustomSprite = useCallback(
    async (input: SaveCustomSpriteInput) => {
      const sprite = await invoke<SpriteInfo>("app_sprites_save_custom", { input });
      await invoke("app_settings_set", {
        key: "active_sprite_id",
        value: sprite.id,
      });
      await load();
      await emit("sprite:changed", { id: sprite.id });
      setState((prev) => ({ ...prev, activeSpriteId: sprite.id }));
      return sprite;
    },
    [load],
  );

  const deleteCustomSprite = useCallback(
    async (spriteId: string) => {
      await invoke("app_sprites_delete", { spriteId });
      const settings = await invoke<Record<string, string>>("app_settings_get");
      await load();
      await emit("sprite:changed", { id: settings.active_sprite_id ?? "dark-cat" });
    },
    [load],
  );

  return {
    activeSpriteId: state.activeSpriteId,
    themeMode: state.themeMode,
    appLanguage: state.appLanguage,
    logLevel: state.logLevel,
    sprites: state.sprites,
    loading: state.loading,
    error: state.error,
    setActiveSpriteId,
    setThemeMode,
    setAppLanguage,
    setLogLevel,
    refresh: load,
    getSpritePrompt,
    getSpriteManifestTemplate,
    loadSpriteManifestFile,
    generateSpriteManifestDraft,
    generateSpriteManifestWithAgent,
    validateSpriteManifest,
    saveCustomSprite,
    deleteCustomSprite,
  };
}
