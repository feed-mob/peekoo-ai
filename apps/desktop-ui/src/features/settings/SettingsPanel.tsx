import { useGlobalSettings } from "./useGlobalSettings";
import { SpriteSelector } from "./SpriteSelector";
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

export function SettingsPanel() {
  const { t } = useTranslation();
  const {
    activeSpriteId,
    themeMode,
    appLanguage,
    sprites,
    loading,
    error,
    setActiveSpriteId,
    setThemeMode,
    setAppLanguage,
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
      </section>
    </div>
  );
}
