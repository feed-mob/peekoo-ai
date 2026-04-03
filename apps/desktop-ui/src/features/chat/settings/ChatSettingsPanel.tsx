import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { SkillToggleList } from "./SkillToggleList";
import { ModelSelector } from "./ModelSelector";
import { AuthSection } from "./AuthSection";
import { useChatSettings } from "./useChatSettings";
import { useTranslation } from "react-i18next";
import { useAgentProviders } from "@/hooks/useAgentProviders";

interface ChatSettingsPanelProps {
  onClose: () => void;
  activeRuntimeName?: string | null;
  configuredModelId?: string | null;
}

export function ChatSettingsPanel({ onClose }: ChatSettingsPanelProps) {
  const { t } = useTranslation();
  const {
    settings,
    catalog,
    selectedProvider,
    isLoading,
    error,
    oauthFlowId,
    oauthStatus,
    oauthError,
    refresh,
    updateSettings,
    saveApiKey,
    setProviderConfig,
    clearAuth,
    startOauth,
    pollOauthStatus,
  } = useChatSettings();

  const { defaultProvider } = useAgentProviders();

  const {
    customModelInput,
    setCustomModelInput,
    compatBaseUrl,
    setCompatBaseUrl,
    maxIterationsInput,
    setMaxIterationsInput,
    apiKey,
    setApiKey,
    authState,
    isCompatibleProvider,
    effectiveSkills,
  } = useChatSettings();

  if (isLoading && !settings) {
    return <div className="text-sm text-text-muted">{t("chatSettings.loading")}</div>;
  }

  if (error) {
    return (
      <div className="space-y-2">
        <p className="text-sm text-danger">{t("chatSettings.failedLoad")}</p>
        {error ? <p className="text-xs text-text-muted">{error}</p> : null}
        <Button size="sm" onClick={() => void refresh()}>
          {t("common.retry")}
        </Button>
      </div>
    );
  }

  if (!settings || !catalog) {
    return (
      <div className="space-y-2">
        <p className="text-sm text-danger">Failed to load settings.</p>
        <Button size="sm" onClick={() => void refresh()}>
          {t("common.retry")}
        </Button>
      </div>
    );
  }

  if (catalog.providers.length === 0) {
    return (
      <div className="space-y-2">
        <p className="text-sm text-text-muted">No ACP runtimes installed.</p>
        <p className="text-xs text-text-secondary">
          Install a runtime from the Settings panel to get started.
        </p>
        <Button size="sm" onClick={() => void refresh()}>
          {t("common.refresh")}
        </Button>
      </div>
    );
  }

  if (!defaultProvider) {
    return (
      <div className="space-y-2">
        <p className="text-sm text-text-muted">Selected runtime not found.</p>
        <p className="text-xs text-text-secondary">
          The previously selected runtime is no longer available.
        </p>
        <Button size="sm" onClick={() => void refresh()}>
          {t("common.refresh")}
        </Button>
      </div>
    );
  }

  return (
    <div className="max-h-[56vh] space-y-4 overflow-y-auto rounded-lg border border-glass-border bg-glass/50 p-3 pr-2">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-semibold text-text-primary">{t("chatSettings.title")}</h3>
        <Button size="sm" variant="ghost" onClick={onClose}>
          {t("common.close")}
        </Button>
      </div>

      <div className="grid grid-cols-1 gap-3">
        <div className="rounded-md border border-glass-border bg-space-surface/40 px-3 py-2 text-xs text-text-muted">
          <div>Active runtime: {defaultProvider.displayName}</div>
        </div>

        {selectedProvider.models.length > 0 ? (
          <ModelSelector
            models={selectedProvider.models}
            value={settings.activeModelId}
            onChange={(modelId) => void updateSettings({ activeModelId: modelId })}
          />
        ) : (
          <label className="flex flex-col gap-1 text-sm text-text-secondary">
            {t("chatSettings.model")}
            <Input
              type="text"
              value={customModelInput}
              onChange={(event) => setCustomModelInput(event.target.value)}
              onBlur={() => {
                if (!customModelInput.trim()) return;
                if (customModelInput.trim() === settings.activeModelId) return;
                void updateSettings({ activeModelId: customModelInput.trim() });
              }}
              placeholder={t("chatSettings.modelPlaceholder")}
              className="bg-space-deep border-glass-border"
            />
          </label>
        )}

        {isCompatibleProvider && (
          <label className="flex flex-col gap-1 text-sm text-text-secondary">
            {t("chatSettings.baseUrl")}
            <Input
              type="text"
              value={compatBaseUrl}
              onChange={(event) => setCompatBaseUrl(event.target.value)}
              onBlur={() => {
                if (!compatBaseUrl.trim()) return;
                void setProviderConfig(settings.activeProviderId, compatBaseUrl.trim());
              }}
              placeholder={t("chatSettings.baseUrlPlaceholder")}
              className="bg-space-deep border-glass-border"
            />
          </label>
        )}

        <label className="flex flex-col gap-1 text-sm text-text-secondary">
          {t("chatSettings.maxToolIterations")}
          <Input
            type="text"
            inputMode="numeric"
            value={maxIterationsInput}
            onChange={(event) => {
              const nextValue = event.target.value.replace(/[^0-9]/g, "");
              setMaxIterationsInput(nextValue);
            }}
            onBlur={() => {
              const value = Number(maxIterationsInput);
              if (!Number.isNaN(value) && value > 0) {
                void updateSettings({ maxToolIterations: value });
                return;
              }
              setMaxIterationsInput(String(settings.maxToolIterations));
            }}
            className="bg-space-deep border-glass-border"
          />
        </label>
      </div>

      <div className="space-y-2">
        <p className="text-sm font-medium text-text-primary">{t("chatSettings.providerAuthentication")}</p>
        <AuthSection
          provider={selectedProvider}
          auth={authState}
          apiKey={apiKey}
          setApiKey={setApiKey}
          oauthFlowRunning={oauthFlowId !== null}
          oauthStatus={oauthStatus}
          oauthError={oauthError}
          onSaveApiKey={async () => {
            if (!apiKey.trim()) return;
            await saveApiKey(settings.activeProviderId, apiKey.trim());
            setApiKey("");
          }}
          onClearAuth={async () => {
            await clearAuth(settings.activeProviderId);
          }}
          onStartOauth={async () => {
            await startOauth(settings.activeProviderId);
          }}
          onCheckOauth={async () => {
            await pollOauthStatus();
          }}
        />
      </div>

      <div className="space-y-2">
        <p className="text-sm font-medium text-text-primary">{t("chatSettings.skills")}</p>
        <SkillToggleList
          skills={effectiveSkills}
          onToggle={(skillId, enabled) => {
            const skills = effectiveSkills.map((skill) =>
              skill.skillId === skillId ? { ...skill, enabled } : skill
            );
            void updateSettings({ skills });
          }}
        />
      </div>
    </div>
  );
}
