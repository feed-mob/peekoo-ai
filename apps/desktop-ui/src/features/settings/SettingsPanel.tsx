import { useGlobalSettings } from "./useGlobalSettings";
import { SpriteSelector } from "./SpriteSelector";
import { CustomSpriteManager } from "./CustomSpriteManager";
import { AgentProviderPanel } from "@/features/agent-runtimes/AgentProviderPanel";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Sun, Moon, Monitor } from "lucide-react";
import { useTranslation } from "react-i18next";
import { setLanguage, type AppLanguage } from "@/lib/i18n";
import { ask } from "@tauri-apps/plugin-dialog";
import { relaunch } from "@tauri-apps/plugin-process";

export function SettingsPanel() {
  const { t } = useTranslation();
  const {
    activeSpriteId,
    themeMode,
    appLanguage,
    logLevel,
    sprites,
    loading,
    error,
    setActiveSpriteId,
    setThemeMode,
    setAppLanguage,
    setLogLevel,
    getSpritePrompt,
    getSpriteManifestTemplate,
    loadSpriteManifestFile,
    generateSpriteManifestDraft,
    generateSpriteManifestWithAgent,
    validateSpriteManifest,
    saveCustomSprite,
    deleteCustomSprite,
  } = useGlobalSettings();

  if (loading) {
    return (
      <div className="flex items-center justify-center h-32 text-text-muted text-sm">
        {t("settings.loading")}
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-32 text-danger text-sm">
        {t("settings.failedLoad", { error })}
      </div>
    );
  }

  const themeOptions = [
    { id: "light", label: t("settings.theme.light"), icon: Sun },
    { id: "dark", label: t("settings.theme.dark"), icon: Moon },
    { id: "system", label: t("settings.theme.system"), icon: Monitor },
  ];

  const logLevelOptions = ["error", "warn", "info", "debug", "trace"] as const;

  async function handleLogLevelChange(nextLevel: string) {
    if (nextLevel === logLevel) {
      return;
    }

    await setLogLevel(nextLevel);
    const shouldRestart = await ask(
      t("settings.logLevelRestartMsg"),
      {
        title: t("settings.restartRequired"),
        kind: "info",
        okLabel: t("settings.restartNow"),
        cancelLabel: t("settings.later"),
      },
    );

    if (shouldRestart) {
      await relaunch();
    }
  }

  return (
    <div className="space-y-8">
      <section className="space-y-3">
        <h3 className="text-sm font-medium text-text-secondary uppercase tracking-wider">
          {t("settings.appearance")}
        </h3>
        <div className="flex gap-2">
          {themeOptions.map((option) => {
            const Icon = option.icon;
            const isActive = themeMode === option.id;
            return (
              <Button
                key={option.id}
                variant={isActive ? "default" : "ghost"}
                size="sm"
                className={`flex-1 flex items-center gap-2 h-10 border ${
                  isActive
                    ? "border-primary/50 shadow-lg shadow-primary/10"
                    : "border-glass-border hover:bg-glass/30 text-text-muted"
                }`}
                onClick={() => void setThemeMode(option.id)}
              >
                <Icon size={16} />
                <span>{option.label}</span>
              </Button>
            );
          })}
        </div>
      </section>

      <section className="space-y-3">
        <h3 className="text-sm font-medium text-text-secondary uppercase tracking-wider">
          {t("settings.language")}
        </h3>
        <Select
          value={appLanguage ?? "en"}
          onValueChange={(value: string) => {
            const language = value as AppLanguage;
            void Promise.all([setAppLanguage(language), setLanguage(language)]);
          }}
        >
          <SelectTrigger className="h-10 text-sm">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="en">{t("settings.languageOptions.en")}</SelectItem>
            <SelectItem value="zh-CN">{t("settings.languageOptions.zhCN")}</SelectItem>
            <SelectItem value="zh-TW">{t("settings.languageOptions.zhTW")}</SelectItem>
            <SelectItem value="ja">{t("settings.languageOptions.ja")}</SelectItem>
            <SelectItem value="es">{t("settings.languageOptions.es")}</SelectItem>
            <SelectItem value="fr">{t("settings.languageOptions.fr")}</SelectItem>
          </SelectContent>
        </Select>
      </section>

      <section className="space-y-3">
        <h3 className="text-sm font-medium text-text-secondary uppercase tracking-wider">
          {t("settings.activePet")}
        </h3>
        <SpriteSelector
          sprites={sprites}
          activeSpriteId={activeSpriteId}
          onSelect={setActiveSpriteId}
        />
        <CustomSpriteManager
          sprites={sprites}
          deleteCustomSprite={deleteCustomSprite}
          getSpritePrompt={getSpritePrompt}
          getSpriteManifestTemplate={getSpriteManifestTemplate}
          loadSpriteManifestFile={loadSpriteManifestFile}
          generateSpriteManifestDraft={generateSpriteManifestDraft}
          generateSpriteManifestWithAgent={generateSpriteManifestWithAgent}
          validateSpriteManifest={validateSpriteManifest}
          saveCustomSprite={saveCustomSprite}
        />
      </section>

      <section className="space-y-3">
        <h3 className="text-sm font-medium text-text-secondary uppercase tracking-wider">{t("settings.logging")}</h3>
        <p className="text-xs text-text-muted">
          {t("settings.loggingHelp")}
        </p>
        <div className="flex flex-wrap gap-2">
          {logLevelOptions.map((option) => {
            const isActive = logLevel === option;
            return (
              <Button
                key={option}
                variant={isActive ? "default" : "ghost"}
                size="sm"
                className={`h-9 border ${
                  isActive
                    ? "border-primary/50 shadow-lg shadow-primary/10"
                    : "border-glass-border hover:bg-glass/30 text-text-muted"
                }`}
                onClick={() => void handleLogLevelChange(option)}
              >
                {option.toUpperCase()}
              </Button>
            );
          })}
        </div>
      </section>

      <section className="space-y-3">
        <h3 className="text-sm font-medium text-text-secondary uppercase tracking-wider">
          {t("settings.acpRuntimes")}
        </h3>
        <AgentProviderPanel />
      </section>
    </div>
  );
}
