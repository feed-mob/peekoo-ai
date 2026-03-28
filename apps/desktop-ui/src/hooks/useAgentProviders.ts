import { invoke } from "@tauri-apps/api/core";
import { useCallback, useState } from "react";
import {
  type ProviderInfo,
  type ProviderConfig,
  type InstallProviderRequest,
  type CustomProviderRequest,
  type PrerequisitesCheck,
  type TestConnectionResult,
  providerInfoSchema,
  prerequisitesCheckSchema,
  testConnectionResultSchema,
  installProviderResponseSchema,
} from "@/types/agent-provider";

export function useAgentProviders() {
  const [providers, setProviders] = useState<ProviderInfo[]>([]);
  const [defaultProvider, setDefaultProvider] = useState<ProviderInfo | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [installingProvider, setInstallingProvider] = useState<string | null>(null);

  // List all providers
  const refresh = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const rawProviders = await invoke<unknown[]>("list_agent_providers");
      const parsed = rawProviders.map((p) => providerInfoSchema.parse(p));
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
    async (req: InstallProviderRequest) => {
      setInstallingProvider(req.providerId);
      try {
        const result = await invoke<{
          success: boolean;
          message: string;
          requiresRestart: boolean;
        }>("install_agent_provider", { req });
        
        const parsed = installProviderResponseSchema.parse(result);
        
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
        await invoke("set_default_provider", { providerId });
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
        await invoke("uninstall_agent_provider", { providerId });
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
      return rawConfig as ProviderConfig;
    } catch (err) {
      setError(String(err));
      throw err;
    }
  }, []);

  // Update provider configuration
  const updateConfig = useCallback(
    async (providerId: string, config: ProviderConfig) => {
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
    async (req: CustomProviderRequest) => {
      try {
        const rawProvider = await invoke<unknown>("add_custom_provider", req);
        const parsed = providerInfoSchema.parse(rawProvider);
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
  };
}
