import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import en from "@/locales/en.json";
import zh from "@/locales/zh.json";
import ja from "@/locales/ja.json";
import es from "@/locales/es.json";
import fr from "@/locales/fr.json";
import zhTW from "@/locales/zh-TW.json";

export type AppLanguage = "en" | "zh-CN" | "zh-TW" | "ja" | "es" | "fr";

const DEFAULT_LANGUAGE: AppLanguage = "en";
const SETTING_APP_LANGUAGE_KEY = "app_language";

function normalizeLanguage(value?: string | null): AppLanguage {
  if (!value) return DEFAULT_LANGUAGE;
  if (value === "zh" || value === "zh-CN" || value === "zh-Hans") {
    return "zh-CN";
  }
  if (value === "zh-TW" || value === "zh-Hant") {
    return "zh-TW";
  }
  if (value === "ja" || value === "ja-JP") {
    return "ja";
  }
  if (value === "es" || value === "es-ES" || value === "es-MX") {
    return "es";
  }
  if (value === "fr" || value === "fr-FR" || value === "fr-CA") {
    return "fr";
  }
  return "en";
}

async function resolveInitialLanguage(): Promise<AppLanguage> {
  try {
    const lang = await invoke<string>("app_settings_get_language");
    return normalizeLanguage(lang);
  } catch {
    try {
      const settings = await invoke<Record<string, string>>("app_settings_get");
      return normalizeLanguage(settings[SETTING_APP_LANGUAGE_KEY]);
    } catch {
      return DEFAULT_LANGUAGE;
    }
  }
}

export async function initI18n(): Promise<void> {
  if (i18n.isInitialized) return;

  const lng = await resolveInitialLanguage();
  await i18n.use(initReactI18next).init({
    resources: {
      en: { translation: en },
      "zh-CN": { translation: zh },
      "zh-TW": { translation: zhTW },
      ja: { translation: ja },
      es: { translation: es },
      fr: { translation: fr },
    },
    lng,
    fallbackLng: DEFAULT_LANGUAGE,
    interpolation: { escapeValue: false },
  });
}

export async function setLanguage(language: string): Promise<void> {
  const normalized = normalizeLanguage(language);
  if (!i18n.isInitialized) {
    await initI18n();
  }
  if (i18n.language !== normalized) {
    await i18n.changeLanguage(normalized);
  }
}

export async function setupLanguageListener(): Promise<() => void> {
  const unlisten = await listen<{ language: string }>("language:changed", (event) => {
    void setLanguage(event.payload.language);
  });
  return unlisten;
}
