import { invoke } from "@tauri-apps/api/core";
import { useCallback, useMemo, useState } from "react";
import {
  type AgentSettings,
  agentSettingsSchema,
  type AgentSettingsCatalog,
  agentSettingsCatalogSchema,
  type ProviderAuth,
  type SkillSettings,
} from "@/types/agent-settings";

type SettingsPatch = {
  activeProviderId?: string;
  activeModelId?: string;
  systemPrompt?: string;
  maxToolIterations?: number;
  skills?: SkillSettings[];
};

export function useChatSettings() {
  const [settings, setSettings] = useState<AgentSettings | null>(null);
  const [catalog, setCatalog] = useState<AgentSettingsCatalog | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [oauthFlowId, setOauthFlowId] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const [rawSettings, rawCatalog] = await Promise.all([
        invoke("agent_settings_get"),
        invoke("agent_settings_catalog"),
      ]);
      setSettings(agentSettingsSchema.parse(rawSettings));
      setCatalog(agentSettingsCatalogSchema.parse(rawCatalog));
    } catch (err) {
      setError(String(err));
    } finally {
      setIsLoading(false);
    }
  }, []);

  const updateSettings = useCallback(async (patch: SettingsPatch) => {
    const rawSettings = await invoke("agent_settings_update", { patch });
    const parsed = agentSettingsSchema.parse(rawSettings);
    setSettings(parsed);
    return parsed;
  }, []);

  const saveApiKey = useCallback(async (providerId: string, apiKey: string) => {
    const rawAuth = await invoke("agent_provider_auth_set_api_key", {
      req: { providerId, apiKey },
    });
    const parsed = rawAuth as ProviderAuth;
    setSettings((prev) => {
      if (!prev) return prev;
      const filtered = prev.providerAuth.filter((item) => item.providerId !== providerId);
      return { ...prev, providerAuth: [...filtered, parsed], version: prev.version + 1 };
    });
    return parsed;
  }, []);

  const clearAuth = useCallback(async (providerId: string) => {
    const rawAuth = await invoke("agent_provider_auth_clear", { req: { providerId } });
    const parsed = rawAuth as ProviderAuth;
    setSettings((prev) => {
      if (!prev) return prev;
      const filtered = prev.providerAuth.filter((item) => item.providerId !== providerId);
      return { ...prev, providerAuth: [...filtered, parsed], version: prev.version + 1 };
    });
    return parsed;
  }, []);

  const startOauth = useCallback(async (providerId: string) => {
    const response = (await invoke("agent_oauth_start", {
      req: { providerId },
    })) as {
      flowId: string;
      authorizeUrl: string;
    };
    setOauthFlowId(response.flowId);
    return response;
  }, []);

  const pollOauthStatus = useCallback(async () => {
    if (!oauthFlowId) return null;
    const response = (await invoke("agent_oauth_status", {
      req: { flowId: oauthFlowId },
    })) as {
      status: string;
      providerAuth?: ProviderAuth;
    };

    if (response.status === "completed" && response.providerAuth) {
      setSettings((prev) => {
        if (!prev) return prev;
        const providerId = response.providerAuth!.providerId;
        const filtered = prev.providerAuth.filter((item) => item.providerId !== providerId);
        return {
          ...prev,
          providerAuth: [...filtered, response.providerAuth!],
          version: prev.version + 1,
        };
      });
      setOauthFlowId(null);
    }

    return response;
  }, [oauthFlowId]);

  const selectedProvider = useMemo(
    () => catalog?.providers.find((provider) => provider.id === settings?.activeProviderId),
    [catalog?.providers, settings?.activeProviderId]
  );

  return {
    settings,
    catalog,
    selectedProvider,
    isLoading,
    error,
    oauthFlowId,
    refresh,
    updateSettings,
    saveApiKey,
    clearAuth,
    startOauth,
    pollOauthStatus,
  };
}
