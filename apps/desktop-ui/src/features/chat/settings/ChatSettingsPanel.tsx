import { useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { AuthSection } from "./AuthSection";
import { ModelSelector } from "./ModelSelector";
import { ProviderSelector } from "./ProviderSelector";
import { SkillToggleList } from "./SkillToggleList";
import { useChatSettings } from "./useChatSettings";

interface ChatSettingsPanelProps {
  onClose: () => void;
}

export function ChatSettingsPanel({ onClose }: ChatSettingsPanelProps) {
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
    clearAuth,
    startOauth,
    pollOauthStatus,
  } = useChatSettings();

  const [apiKey, setApiKey] = useState("");
  const [maxIterationsInput, setMaxIterationsInput] = useState("50");

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    if (!settings) return;
    setMaxIterationsInput(String(settings.maxToolIterations));
  }, [settings?.maxToolIterations]);

  const authState = useMemo(() => {
    if (!settings) return undefined;
    return settings.providerAuth.find((item) => item.providerId === settings.activeProviderId);
  }, [settings]);

  const effectiveSkills = useMemo(() => {
    if (!settings || !catalog) return [];
    return settings.skills.length > 0 ? settings.skills : catalog.discoveredSkills;
  }, [catalog, settings]);

  useEffect(() => {
    if (!settings || !selectedProvider) return;
    if (!selectedProvider.models.includes(settings.activeModelId)) {
      const nextModel = selectedProvider.models[0];
      if (nextModel) {
        void updateSettings({
          activeProviderId: settings.activeProviderId,
          activeModelId: nextModel,
        });
      }
    }
  }, [selectedProvider, settings, updateSettings]);

  if (isLoading && !settings) {
    return <div className="text-sm text-text-muted">Loading settings...</div>;
  }

  if (!settings || !catalog || !selectedProvider) {
    return (
      <div className="space-y-2">
        <p className="text-sm text-danger">Failed to load settings.</p>
        {error ? <p className="text-xs text-text-muted">{error}</p> : null}
        <Button size="sm" onClick={() => void refresh()}>
          Retry
        </Button>
      </div>
    );
  }

  return (
    <div className="max-h-[56vh] space-y-4 overflow-y-auto rounded-lg border border-glass-border bg-glass/50 p-3 pr-2">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-semibold text-text-primary">Agent Settings</h3>
        <Button size="sm" variant="ghost" onClick={onClose}>
          Close
        </Button>
      </div>

      <div className="grid grid-cols-1 gap-3">
        <ProviderSelector
          providers={catalog.providers}
          value={settings.activeProviderId}
          onChange={(providerId) => {
            const provider = catalog.providers.find((entry) => entry.id === providerId);
            const nextModel = provider?.models[0];
            const patch = nextModel
              ? { activeProviderId: providerId, activeModelId: nextModel }
              : { activeProviderId: providerId };
            void updateSettings(patch);
          }}
        />

        <ModelSelector
          models={selectedProvider.models}
          value={settings.activeModelId}
          onChange={(modelId) => void updateSettings({ activeModelId: modelId })}
        />

        <label className="flex flex-col gap-1 text-sm text-text-secondary">
          Max Tool Iterations
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
        <p className="text-sm font-medium text-text-primary">Provider Authentication</p>
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
        <p className="text-sm font-medium text-text-primary">Skills</p>
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
