import { UpdatePromptDialog } from "@/features/about/UpdatePromptDialog";
import { checkForAppUpdates, installAppUpdate, type AppUpdateInfo } from "@/lib/updater";
import { useUpdateInstallProgress } from "@/lib/update-install-progress";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState } from "react";

const FORCE_UPDATER_IN_DEV = import.meta.env.DEV && import.meta.env.VITE_FORCE_UPDATER_DIALOG === "true";

export default function UpdaterView() {
  const [updateInfo, setUpdateInfo] = useState<AppUpdateInfo | null>(null);
  const [isInstallingUpdate, setIsInstallingUpdate] = useState(false);
  const [updateError, setUpdateError] = useState<string | null>(null);
  const progress = useUpdateInstallProgress({ isInstalling: isInstallingUpdate });

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
    progress.start();

    try {
      await installAppUpdate(updateInfo.update, progress.handleEvent);
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      setUpdateError(message);
      setIsInstallingUpdate(false);
      progress.reset();
    }
  }

  async function closeUpdaterPanel() {
    if (isInstallingUpdate) {
      return;
    }

    await getCurrentWindow().close();
  }

  return (
    <div className="h-screen w-screen bg-space-void/90">
      <UpdatePromptDialog
        updateInfo={updateInfo}
        isInstalling={isInstallingUpdate}
        installError={updateError}
        installPhase={progress.installPhase}
        downloadedBytes={progress.downloadedBytes}
        totalBytes={progress.totalBytes}
        progressPercent={progress.progressPercent}
        etaSeconds={progress.etaSeconds}
        onInstall={() => void handleInstallUpdate()}
        onLater={() => void closeUpdaterPanel()}
      />
    </div>
  );
}
