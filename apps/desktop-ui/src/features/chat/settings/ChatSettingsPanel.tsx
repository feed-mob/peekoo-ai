import { useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { AuthSection } from "./AuthSection";
import { ModelSelector } from "./ModelSelector";
import { ProviderSelector } from "./ProviderSelector";
import { SkillToggleList } from "./SkillToggleList";
import { useChatSettings } from "./useChatSettings";
import { useAgentProviders } from "@/hooks/useAgentProviders";

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
  const { getRuntimeDefaults } = useAgentProviders();

  const [apiKey, setApiKey] = useState("");
  const [maxIterationsInput, setMaxIterationsInput] = useState("50");
  const [customModelInput, setCustomModelInput] = useState("");
  const [runtimeProviderSummary, setRuntimeProviderSummary] = useState<string | null>(null);
  const [runtimeModelSummary, setRuntimeModelSummary] = useState<string | null>(null);

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
  const supportsAuth = (selectedProvider?.authModes.length ?? 0) > 0;

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

  useEffect(() => {
    if (!settings) return;
    setCustomModelInput(settings.activeModelId);
  }, [settings?.activeModelId, settings?.activeProviderId]);

  useEffect(() => {
    let cancelled = false;

    if (!settings?.activeProviderId) {
      setRuntimeProviderSummary(null);
      setRuntimeModelSummary(null);
      return;
    }

    void getRuntimeDefaults(settings.activeProviderId)
      .then(({ model }) => {
        if (!cancelled) {
          // LLM provider is now discovered via ACP, not stored separately
          setRuntimeProviderSummary(null);
          setRuntimeModelSummary(model?.displayName ?? model?.modelId ?? null);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setRuntimeProviderSummary(null);
          setRuntimeModelSummary(null);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [getRuntimeDefaults, settings?.activeProviderId]);

  if (isLoading && !settings) {
    return <div className="text-sm text-text-muted">Loading settings...</div>;
  }

  if (error) {
    return (
      <div className="space-y-2">
        <p className="text-sm text-danger">Failed to load settings.</p>
        <p className="text-xs text-text-muted">{error}</p>
        <Button size="sm" onClick={() => void refresh()}>
          Retry
        </Button>
      </div>
    );
  }

  if (!settings || !catalog) {
    return (
      <div className="space-y-2">
        <p className="text-sm text-danger">Failed to load settings.</p>
        <Button size="sm" onClick={() => void refresh()}>
          Retry
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
          Refresh
        </Button>
      </div>
    );
  }

  if (!selectedProvider) {
    return (
      <div className="space-y-2">
        <p className="text-sm text-text-muted">Selected runtime not found.</p>
        <p className="text-xs text-text-secondary">
          The previously selected runtime is no longer available.
        </p>
        <Button size="sm" onClick={() => void refresh()}>
          Refresh
        </Button>
      </div>
    );
  }

  return (
    <div className="max-h-[56vh] space-y-4 overflow-y-auto rounded-lg border border-glass-border bg-glass/50 p-3 pr-2">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-semibold text-text-primary">Runtime Settings</h3>
        <Button size="sm" variant="ghost" onClick={onClose}>
          Close
        </Button>
      </div>

      <div className="grid grid-cols-1 gap-3">
        {(runtimeProviderSummary || runtimeModelSummary) && (
          <div className="rounded-md border border-glass-border bg-space-surface/40 px-3 py-2 text-xs text-text-muted">
            <div>
              Runtime default LLM provider: {runtimeProviderSummary ?? "Not configured"}
            </div>
            <div>Runtime default model: {runtimeModelSummary ?? "Not configured"}</div>
          </div>
        )}

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

        {selectedProvider.models.length > 0 ? (
          <ModelSelector
            models={selectedProvider.models}
            value={settings.activeModelId}
            onChange={(modelId) => void updateSettings({ activeModelId: modelId })}
          />
        ) : (
          <label className="flex flex-col gap-1 text-sm text-text-secondary">
            Model
            <Input
              type="text"
              value={customModelInput}
              onChange={(event) => setCustomModelInput(event.target.value)}
              onBlur={() => {
                if (!customModelInput.trim()) return;
                if (customModelInput.trim() === settings.activeModelId) return;
                void updateSettings({ activeModelId: customModelInput.trim() });
              }}
              placeholder="e.g. gpt-4.1-mini"
              className="bg-space-deep border-glass-border"
            />
          </label>
        )}

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

      {supportsAuth && (
        <div className="space-y-2">
          <p className="text-sm font-medium text-text-primary">Runtime Authentication</p>
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
      )}

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
