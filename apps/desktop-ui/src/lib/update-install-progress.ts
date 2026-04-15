import { useCallback, useMemo, useState } from "react";
import type { DownloadEvent } from "@tauri-apps/plugin-updater";

export type InstallPhase = "idle" | "downloading" | "installing";

interface UseUpdateInstallProgressOptions {
  isInstalling: boolean;
}

export interface UpdateInstallProgressState {
  installPhase: InstallPhase;
  downloadedBytes: number;
  totalBytes: number | null;
  progressPercent: number | null;
  etaSeconds: number | null;
}

export interface UpdateInstallProgressController extends UpdateInstallProgressState {
  reset: () => void;
  start: () => void;
  handleEvent: (event: DownloadEvent) => void;
}

export function useUpdateInstallProgress(
  options: UseUpdateInstallProgressOptions,
): UpdateInstallProgressController {
  const [installPhase, setInstallPhase] = useState<InstallPhase>("idle");
  const [downloadedBytes, setDownloadedBytes] = useState(0);
  const [totalBytes, setTotalBytes] = useState<number | null>(null);
  const [downloadStartMs, setDownloadStartMs] = useState<number | null>(null);

  const reset = useCallback(() => {
    setInstallPhase("idle");
    setDownloadedBytes(0);
    setTotalBytes(null);
    setDownloadStartMs(null);
  }, []);

  const start = useCallback(() => {
    setInstallPhase("downloading");
    setDownloadedBytes(0);
    setTotalBytes(null);
    setDownloadStartMs(Date.now());
  }, []);

  const handleEvent = useCallback((event: DownloadEvent) => {
    switch (event.event) {
      case "Started": {
        setInstallPhase("downloading");
        setDownloadedBytes(0);
        setTotalBytes(event.data.contentLength ?? null);
        setDownloadStartMs(Date.now());
        break;
      }
      case "Progress": {
        setInstallPhase("downloading");
        setDownloadedBytes((current) => current + event.data.chunkLength);
        break;
      }
      case "Finished": {
        setInstallPhase("installing");
        setDownloadStartMs(null);
        break;
      }
    }
  }, []);

  const progressPercent = useMemo(() => {
    if (!totalBytes || totalBytes <= 0) {
      return null;
    }

    return Math.min(100, Math.round((downloadedBytes / totalBytes) * 100));
  }, [downloadedBytes, totalBytes]);

  const etaSeconds = useMemo(() => {
    if (!options.isInstalling || installPhase !== "downloading") {
      return null;
    }

    if (!totalBytes || totalBytes <= 0 || downloadedBytes <= 0 || !downloadStartMs) {
      return null;
    }

    const elapsedMs = Date.now() - downloadStartMs;
    if (elapsedMs < 1000) {
      return null;
    }

    const bytesPerSecond = downloadedBytes / (elapsedMs / 1000);
    if (!Number.isFinite(bytesPerSecond) || bytesPerSecond <= 0) {
      return null;
    }

    const remainingBytes = totalBytes - downloadedBytes;
    if (remainingBytes <= 0) {
      return 0;
    }

    return Math.max(0, Math.round(remainingBytes / bytesPerSecond));
  }, [downloadStartMs, downloadedBytes, installPhase, options.isInstalling, totalBytes]);

  return {
    installPhase,
    downloadedBytes,
    totalBytes,
    progressPercent,
    etaSeconds,
    reset,
    start,
    handleEvent,
  };
}
