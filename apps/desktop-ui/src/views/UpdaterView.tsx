import { UpdatePromptDialog } from "@/features/about/UpdatePromptDialog";
import { checkForAppUpdates, installAppUpdate, type AppUpdateInfo } from "@/lib/updater";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState } from "react";

const FORCE_UPDATER_IN_DEV = import.meta.env.DEV && import.meta.env.VITE_FORCE_UPDATER_DIALOG === "true";

export default function UpdaterView() {
  const [updateInfo, setUpdateInfo] = useState<AppUpdateInfo | null>(null);
  const [isInstallingUpdate, setIsInstallingUpdate] = useState(false);
  const [updateError, setUpdateError] = useState<string | null>(null);
  const [downloadedBytes, setDownloadedBytes] = useState(0);
  const [totalBytes, setTotalBytes] = useState<number | null>(null);
  const [installPhase, setInstallPhase] = useState<"idle" | "downloading" | "installing">("idle");
  const [downloadStartMs, setDownloadStartMs] = useState<number | null>(null);

  useEffect(() => {
    let active = true;

    void checkForAppUpdates({ forceInDev: FORCE_UPDATER_IN_DEV }).then((nextUpdateInfo) => {
      if (!active) {
        return;
      }

      if (!nextUpdateInfo) {
        void getCurrentWindow().close();
        return;
      }

      setUpdateInfo(nextUpdateInfo);
      setUpdateError(null);
    });

    return () => {
      active = false;
    };
  }, []);

  async function handleInstallUpdate() {
    if (!updateInfo) {
      return;
    }

    if (updateInfo.update === null) {
      setUpdateError("Dev mock mode: install is disabled.");
      return;
    }

    setIsInstallingUpdate(true);
    setUpdateError(null);
    setDownloadedBytes(0);
    setTotalBytes(null);
    setInstallPhase("downloading");

    try {
      await installAppUpdate(updateInfo.update, (event) => {
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
      });
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      setUpdateError(message);
      setIsInstallingUpdate(false);
      setInstallPhase("idle");
      setDownloadStartMs(null);
    }
  }

  async function closeUpdaterPanel() {
    if (isInstallingUpdate) {
      return;
    }

    await getCurrentWindow().close();
  }

  const progressPercent = totalBytes && totalBytes > 0
    ? Math.min(100, Math.round((downloadedBytes / totalBytes) * 100))
    : null;

  const etaSeconds = (() => {
    if (!isInstallingUpdate || installPhase !== "downloading") {
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
  })();

  return (
    <div className="h-screen w-screen bg-space-void/90">
      <UpdatePromptDialog
        updateInfo={updateInfo}
        isInstalling={isInstallingUpdate}
        installError={updateError}
        installPhase={installPhase}
        downloadedBytes={downloadedBytes}
        totalBytes={totalBytes}
        progressPercent={progressPercent}
        etaSeconds={etaSeconds}
        onInstall={() => void handleInstallUpdate()}
        onLater={() => void closeUpdaterPanel()}
      />
    </div>
  );
}
