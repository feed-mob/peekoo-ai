import { useMemo } from "react";
import { Button } from "@/components/ui/button";
import { InstallProgressCard } from "@/components/update/InstallProgressCard";
import { ReleaseNotesMarkdown } from "@/components/update/ReleaseNotesMarkdown";
import { invoke } from "@tauri-apps/api/core";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import type { AppUpdateInfo } from "@/lib/updater";
import type { InstallPhase } from "@/lib/update-install-progress";
import { useTranslation } from "react-i18next";

interface UpdatePromptDialogProps {
  updateInfo: AppUpdateInfo | null;
  isInstalling: boolean;
  installError: string | null;
  installPhase: InstallPhase;
  downloadedBytes: number;
  totalBytes: number | null;
  progressPercent: number | null;
  etaSeconds: number | null;
  onInstall: () => void;
  onLater: () => void;
}

export function UpdatePromptDialog({
  updateInfo,
  isInstalling,
  installError,
  installPhase,
  downloadedBytes,
  totalBytes,
  progressPercent,
  etaSeconds,
  onInstall,
  onLater,
}: UpdatePromptDialogProps) {
  const { t } = useTranslation();

  const fallbackMessage = useMemo(() => {
    if (!updateInfo) {
      return "";
    }

    return t("updater.message", { version: updateInfo.version });
  }, [t, updateInfo]);

  async function openFullChangelog() {
    if (!updateInfo) {
      return;
    }

    try {
      await invoke("system_open_url", { url: updateInfo.releaseUrl });
    } catch (error) {
      console.error("Failed to open release page:", error);
    }
  }

  return (
    <Dialog
      open={updateInfo !== null}
      onOpenChange={(open) => {
        if (!open && updateInfo) {
          onLater();
        }
      }}
    >
      <DialogContent className="max-w-2xl border border-glass-border bg-space-void/95 p-0 text-text-primary backdrop-blur-xl">
        <DialogHeader className="space-y-2 border-b border-glass-border/70 px-6 py-5">
          <DialogTitle className="text-xl text-text-primary">{t("updater.title")}</DialogTitle>
          <DialogDescription className="text-sm text-text-muted">
            {updateInfo ? t("updater.versionAvailable", { version: updateInfo.version }) : ""}
          </DialogDescription>
        </DialogHeader>

        <div className="max-h-[52vh] overflow-y-auto px-6 py-4 custom-scrollbar">
          {isInstalling ? (
            <InstallProgressCard
              className="mb-4 rounded-lg border border-glass-border/70 bg-space-overlay/45 px-3 py-3"
              installPhase={installPhase}
              downloadedBytes={downloadedBytes}
              totalBytes={totalBytes}
              progressPercent={progressPercent}
              etaSeconds={etaSeconds}
            />
          ) : null}

          {updateInfo?.body ? (
            <ReleaseNotesMarkdown notes={updateInfo.body} />
          ) : (
            <p className="text-sm leading-6 text-text-primary">{fallbackMessage}</p>
          )}
          {installError ? (
            <p className="mt-4 rounded-lg border border-danger/40 bg-danger/10 px-3 py-2 text-sm text-danger">
              {installError}
            </p>
          ) : null}
        </div>

        <DialogFooter className="gap-2 border-t border-glass-border/70 px-6 py-4 sm:justify-end">
          <Button variant="ghost" onClick={() => void openFullChangelog()} disabled={!updateInfo || isInstalling}>
            {t("updater.viewFullChangelog")}
          </Button>
          <Button variant="glass" onClick={onLater} disabled={isInstalling}>
            {t("updater.later")}
          </Button>
          <Button variant="success" onClick={onInstall} disabled={isInstalling || !updateInfo || !updateInfo.update}>
            {isInstalling ? t("updater.installing") : t("updater.installButton")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
