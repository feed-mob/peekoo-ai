import { invoke } from "@tauri-apps/api/core";
import { useCallback, useState } from "react";
import {
  type RuntimeInfo,
  type RuntimeConfig,
  type InstallRuntimeRequest,
  type CustomRuntimeRequest,
  type RuntimeLlmProviderInfo,
  type RuntimeLlmProviderUpsert,
  type RuntimeModelInfo,
  type RuntimeModelUpsert,
  runtimeInfoSchema,
  runtimeLlmProviderInfoSchema,
  runtimeModelInfoSchema,
  prerequisitesCheckSchema,
  testConnectionResultSchema,
  installRuntimeResponseSchema,
} from "@/types/agent-runtime";

export function useAgentProviders() {
  const [providers, setProviders] = useState<RuntimeInfo[]>([]);
  const [defaultProvider, setDefaultProvider] = useState<RuntimeInfo | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [installingProvider, setInstallingProvider] = useState<string | null>(null);

  // List all providers
  const refresh = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const rawProviders = await invoke<unknown[]>("list_agent_runtimes");
      const parsed = rawProviders.map((p) => runtimeInfoSchema.parse(p));
      setProviders(parsed);
      setDefaultProvider(parsed.find((p) => p.isDefault) || null);
    } catch (err) {
      setError(String(err));
    } finally {
      setIsLoading(false);
    }
  }, []);

  // Install a provider
  const installProvider = useCallback(
    async (req: InstallRuntimeRequest) => {
      setInstallingProvider(req.providerId);
      try {
        const result = await invoke<{
          success: boolean;
          message: string;
          requiresRestart: boolean;
        }>("install_agent_runtime", { req });
        
        const parsed = installRuntimeResponseSchema.parse(result);
        
        if (parsed.success) {
          await refresh();
        }
        
        return parsed;
      } catch (err) {
        setError(String(err));
        throw err;
      } finally {
        setInstallingProvider(null);
      }
    },
    [refresh]
  );

  // Set default provider
  const setAsDefault = useCallback(
    async (providerId: string) => {
      try {
        await invoke("set_default_agent_runtime", { runtimeId: providerId });
        await refresh();
      } catch (err) {
        setError(String(err));
        throw err;
      }
    },
    [refresh]
  );

  // Uninstall a provider
  const uninstallProvider = useCallback(
    async (providerId: string) => {
      try {
        await invoke("uninstall_agent_runtime", { runtimeId: providerId });
        await refresh();
      } catch (err) {
        setError(String(err));
        throw err;
      }
    },
    [refresh]
  );

  // Get provider configuration
  const getConfig = useCallback(async (providerId: string) => {
    try {
      const rawConfig = await invoke<unknown>("get_provider_config", { providerId });
      return rawConfig as RuntimeConfig;
    } catch (err) {
      setError(String(err));
      throw err;
    }
  }, []);

  // Update provider configuration
  const updateConfig = useCallback(
    async (providerId: string, config: RuntimeConfig) => {
      try {
        await invoke("update_provider_config", { providerId, config });
        await refresh();
      } catch (err) {
        setError(String(err));
        throw err;
      }
    },
    [refresh]
  );

  // Test provider connection
  const testConnection = useCallback(async (providerId: string) => {
    try {
      const rawResult = await invoke<unknown>("test_provider_connection", { providerId });
      return testConnectionResultSchema.parse(rawResult);
    } catch (err) {
      setError(String(err));
      throw err;
    }
  }, []);

  // Check installation prerequisites
  const checkPrerequisites = useCallback(async (method: string) => {
    try {
      const rawCheck = await invoke<unknown>("check_installation_prerequisites", { method });
      return prerequisitesCheckSchema.parse(rawCheck);
    } catch (err) {
      setError(String(err));
      throw err;
    }
  }, []);

  // Add custom provider
  const addCustomProvider = useCallback(
    async (req: CustomRuntimeRequest) => {
      try {
        const rawProvider = await invoke<unknown>("add_custom_provider", req);
        const parsed = runtimeInfoSchema.parse(rawProvider);
        await refresh();
        return parsed;
      } catch (err) {
        setError(String(err));
        throw err;
      }
    },
    [refresh]
  );

  // Remove custom provider
  const removeCustomProvider = useCallback(
    async (providerId: string) => {
      try {
        await invoke("remove_custom_provider", { providerId });
        await refresh();
      } catch (err) {
        setError(String(err));
        throw err;
      }
    },
    [refresh]
  );

  // Get installed providers
  const installedProviders = providers.filter((p) => p.isInstalled);

  // Get available providers (not installed)
  const availableProviders = providers.filter((p) => !p.isInstalled);

  const listRuntimeProviders = useCallback(async (runtimeId: string) => {
    try {
      const raw = await invoke<unknown[]>("list_runtime_llm_providers", { runtimeId });
      return raw.map((item) => runtimeLlmProviderInfoSchema.parse(item));
    } catch (err) {
      setError(String(err));
      throw err;
    }
  }, []);

  const saveRuntimeProvider = useCallback(
    async (runtimeId: string, provider: RuntimeLlmProviderUpsert): Promise<RuntimeLlmProviderInfo> => {
      try {
        const raw = await invoke<unknown>("upsert_runtime_llm_provider", { runtimeId, provider });
        await refresh();
        return runtimeLlmProviderInfoSchema.parse(raw);
      } catch (err) {
        setError(String(err));
        throw err;
      }
    },
    [refresh]
  );

  const listRuntimeModels = useCallback(async (runtimeId: string) => {
    try {
      const raw = await invoke<unknown[]>("list_runtime_models", { runtimeId });
      return raw.map((item) => runtimeModelInfoSchema.parse(item));
    } catch (err) {
      setError(String(err));
      throw err;
    }
  }, []);

  const saveRuntimeModel = useCallback(
    async (runtimeId: string, model: RuntimeModelUpsert): Promise<RuntimeModelInfo> => {
      try {
        const raw = await invoke<unknown>("upsert_runtime_model", { runtimeId, model });
        await refresh();
        return runtimeModelInfoSchema.parse(raw);
      } catch (err) {
        setError(String(err));
        throw err;
      }
    },
    [refresh]
  );

  const getRuntimeDefaults = useCallback(
    async (runtimeId: string) => {
      const [providers, models] = await Promise.all([
        listRuntimeProviders(runtimeId),
        listRuntimeModels(runtimeId),
      ]);

      return {
        provider: providers.find((item) => item.isDefault) ?? null,
        model: models.find((item) => item.isDefault) ?? null,
      };
    },
    [listRuntimeModels, listRuntimeProviders]
  );

  return {
    providers,
    installedProviders,
    availableProviders,
    defaultProvider,
    isLoading,
    installingProvider,
    error,
    refresh,
    installProvider,
    setAsDefault,
    uninstallProvider,
    getConfig,
    updateConfig,
    testConnection,
    checkPrerequisites,
    addCustomProvider,
    removeCustomProvider,
    listRuntimeProviders,
    saveRuntimeProvider,
    listRuntimeModels,
    saveRuntimeModel,
    getRuntimeDefaults,
  };
}
