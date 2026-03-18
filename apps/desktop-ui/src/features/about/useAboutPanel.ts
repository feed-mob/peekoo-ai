import { useCallback, useEffect, useState } from "react";
import { getName, getVersion } from "@tauri-apps/api/app";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { loadAboutSnapshot, type AboutSnapshot } from "./about-state";

interface AboutPanelState {
  snapshot: AboutSnapshot | null;
  loading: boolean;
  checking: boolean;
  installing: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  installUpdate: () => Promise<void>;
}

function getErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  return "Unknown error";
}

export function useAboutPanel(): AboutPanelState {
  const [snapshot, setSnapshot] = useState<AboutSnapshot | null>(null);
  const [update, setUpdate] = useState<Update | null>(null);
  const [loading, setLoading] = useState(true);
  const [checking, setChecking] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setChecking(true);
    setError(null);

    try {
      const result = await loadAboutSnapshot({
        getName,
        getVersion,
        check: import.meta.env.PROD ? check : async () => null,
      });

      setSnapshot(result.snapshot);
      setUpdate(import.meta.env.PROD ? (result.update as Update | null) : null);
    } catch (nextError) {
      setError(getErrorMessage(nextError));
    } finally {
      setLoading(false);
      setChecking(false);
    }
  }, []);

  const installUpdate = useCallback(async () => {
    if (!update) {
      return;
    }

    setInstalling(true);
    setError(null);

    try {
      await update.downloadAndInstall();
      await relaunch();
    } catch (nextError) {
      setError(getErrorMessage(nextError));
    } finally {
      setInstalling(false);
    }
  }, [update]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return {
    snapshot,
    loading,
    checking,
    installing,
    error,
    refresh,
    installUpdate,
  };
}
