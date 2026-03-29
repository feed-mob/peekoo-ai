import { invoke } from "@tauri-apps/api/core";
import { useCallback, useState } from "react";
import {
  type RuntimeInfo,
  type RuntimeConfig,
  type InstallRuntimeRequest,
  type CustomRuntimeRequest,
  type RuntimeInspectionResult,
  runtimeInfoSchema,
  runtimeInspectionResultSchema,
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

  // Inspect runtime capabilities via ACP protocol
  const inspectRuntime = useCallback(async (runtimeId: string): Promise<RuntimeInspectionResult> => {
    try {
      const raw = await invoke<unknown>("inspect_runtime", { runtimeId });
      return runtimeInspectionResultSchema.parse(raw);
    } catch (err) {
      setError(String(err));
      throw err;
    }
  }, []);

  // Authenticate with a runtime using the specified auth method
  const authenticateRuntime = useCallback(async (runtimeId: string, methodId: string): Promise<void> => {
    try {
      await invoke("authenticate_runtime", { runtimeId, methodId });
    } catch (err) {
      setError(String(err));
      throw err;
    }
  }, []);

  // Refresh runtime capabilities (re-inspect)
  const refreshRuntimeCapabilities = useCallback(async (runtimeId: string): Promise<RuntimeInspectionResult> => {
    try {
      const raw = await invoke<unknown>("refresh_runtime_capabilities", { runtimeId });
      return runtimeInspectionResultSchema.parse(raw);
    } catch (err) {
      setError(String(err));
      throw err;
    }
  }, []);

  // Legacy: Get runtime defaults (now derived from inspection)
  // This maintains backward compatibility with components that expect the old API
  const getRuntimeDefaults = useCallback(
    async (runtimeId: string): Promise<{ 
      provider: { providerId: string; displayName: string | null } | null; 
      model: { modelId: string; displayName: string | null } | null 
    }> => {
      // Find the provider so we can avoid inspecting runtimes before metadata loads.
      const provider = providers.find(p => p.providerId === runtimeId);

      // Skip inspection until provider metadata is loaded.
      if (!provider || provider.isBundled) {
        return { provider: null, model: null };
      }
      
      try {
        const inspection = await inspectRuntime(runtimeId);
        
        // Map discovered models to legacy format
        const model = inspection.discoveredModels.find(
          m => m.modelId === inspection.currentModelId
        ) || inspection.discoveredModels[0];
        
        return {
          provider: null, // No longer storing LLM providers separately
          model: model ? {
            modelId: model.modelId,
            displayName: model.name,
          } : null,
        };
      } catch {
        return { provider: null, model: null };
      }
    },
    [inspectRuntime, providers]
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
    inspectRuntime,
    authenticateRuntime,
    refreshRuntimeCapabilities,
    getRuntimeDefaults,
  };
}
