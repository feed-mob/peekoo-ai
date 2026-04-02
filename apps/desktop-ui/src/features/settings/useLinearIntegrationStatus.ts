import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

const POLL_INTERVAL_MS = 15000;

type UiStatus =
  | "uninstalled"
  | "disabled"
  | "disconnected"
  | "connecting"
  | "connected"
  | "syncing"
  | "error"
  | "unknown";

interface PluginSummary {
  pluginKey: string;
  enabled: boolean;
}

interface PluginConnectionStatus {
  connected?: boolean;
  status?: string;
  workspaceName?: string | null;
  userName?: string | null;
  userEmail?: string | null;
  lastSyncAt?: string | null;
  lastError?: string | null;
}

export interface LinearIntegrationStatus {
  uiStatus: UiStatus;
  installed: boolean;
  enabled: boolean;
  connected: boolean;
  workspaceName: string | null;
  userName: string | null;
  userEmail: string | null;
  lastSyncAt: string | null;
  lastError: string | null;
}

const DEFAULT_STATUS: LinearIntegrationStatus = {
  uiStatus: "uninstalled",
  installed: false,
  enabled: false,
  connected: false,
  workspaceName: null,
  userName: null,
  userEmail: null,
  lastSyncAt: null,
  lastError: null,
};

function mapPluginStatus(status: string | undefined, connected: boolean): UiStatus {
  switch (status) {
    case "connected":
      return connected ? "connected" : "disconnected";
    case "connecting":
      return "connecting";
    case "syncing":
      return "syncing";
    case "error":
      return "error";
    case "disconnected":
      return "disconnected";
    default:
      return connected ? "connected" : "unknown";
  }
}

export function useLinearIntegrationStatus() {
  const [status, setStatus] = useState<LinearIntegrationStatus>(DEFAULT_STATUS);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      setError(null);

      const plugins = await invoke<PluginSummary[]>("plugins_list");
      const linear = plugins.find((plugin) => plugin.pluginKey === "linear");

      if (!linear) {
        setStatus(DEFAULT_STATUS);
        setIsLoading(false);
        return;
      }

      if (!linear.enabled) {
        setStatus({
          ...DEFAULT_STATUS,
          uiStatus: "disabled",
          installed: true,
          enabled: false,
        });
        setIsLoading(false);
        return;
      }

      const raw = await invoke<string>("plugin_query_data", {
        pluginKey: "linear",
        providerName: "connection_status",
      });

      const parsed = JSON.parse(raw) as PluginConnectionStatus;
      const connected = Boolean(parsed.connected);

      setStatus({
        uiStatus: mapPluginStatus(parsed.status, connected),
        installed: true,
        enabled: true,
        connected,
        workspaceName: parsed.workspaceName ?? null,
        userName: parsed.userName ?? null,
        userEmail: parsed.userEmail ?? null,
        lastSyncAt: parsed.lastSyncAt ?? null,
        lastError: parsed.lastError ?? null,
      });
    } catch (err) {
      setError(String(err));
      setStatus((prev) => ({
        ...prev,
        uiStatus: prev.installed ? "error" : "unknown",
      }));
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
    const interval = window.setInterval(() => {
      void refresh();
    }, POLL_INTERVAL_MS);

    return () => {
      window.clearInterval(interval);
    };
  }, [refresh]);

  return {
    status,
    isLoading,
    error,
    refresh,
  };
}
