import { useMemo } from "react";
import { Streamdown } from "streamdown";
import { Button } from "@/components/ui/button";
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
import { useTranslation } from "react-i18next";

interface UpdatePromptDialogProps {
  updateInfo: AppUpdateInfo | null;
  isInstalling: boolean;
  installError: string | null;
  installPhase: "idle" | "downloading" | "installing";
  downloadedBytes: number;
  totalBytes: number | null;
  progressPercent: number | null;
  etaSeconds: number | null;
  onInstall: () => void;
  onLater: () => void;
}

function formatBytes(value: number): string {
  if (value < 1024) {
    return `${value} B`;
  }

  const units = ["KB", "MB", "GB"];
  let size = value / 1024;
  let unitIndex = 0;

  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex += 1;
  }

  return `${size.toFixed(1)} ${units[unitIndex]}`;
}

function formatDuration(seconds: number): string {
  if (seconds < 60) {
    return `${seconds}s`;
  }

  const minutes = Math.floor(seconds / 60);
  const remain = seconds % 60;
  return `${minutes}m ${remain}s`;
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
            <div className="mb-4 rounded-lg border border-glass-border/70 bg-space-overlay/45 px-3 py-3">
              <div className="mb-2 flex items-center justify-between gap-2 text-xs text-text-muted">
                <span>
                  {installPhase === "installing"
                    ? t("updater.installingFiles")
                    : t("updater.downloading")}
                </span>
                <span>
                  {progressPercent !== null ? `${progressPercent}%` : t("updater.calculating")}
                </span>
              </div>
              <div className="h-2 w-full overflow-hidden rounded-full bg-space-deep/90">
                <div
                  className="h-full rounded-full bg-gradient-primary transition-[width] duration-200"
                  style={{ width: `${progressPercent ?? 15}%` }}
                />
              </div>
              {installPhase !== "installing" ? (
                <div className="mt-2 space-y-1">
                  <p className="text-xs text-text-muted">
                    {totalBytes
                      ? t("updater.downloadedDetail", {
                          downloaded: formatBytes(downloadedBytes),
                          total: formatBytes(totalBytes),
                        })
                      : t("updater.downloadedOnly", { downloaded: formatBytes(downloadedBytes) })}
                  </p>
                  {etaSeconds !== null ? (
                    <p className="text-xs text-text-muted">
                      {t("updater.eta", { eta: formatDuration(etaSeconds) })}
                    </p>
                  ) : null}
                </div>
              ) : (
                <p className="mt-2 text-xs text-text-muted">{t("updater.restartSoon")}</p>
              )}
            </div>
          ) : null}

          {updateInfo?.body ? (
            <div className="text-sm leading-6 text-text-primary [&_h2]:mt-4 [&_h2]:text-base [&_h2]:font-semibold [&_h2]:text-text-primary [&_h3]:mt-3 [&_h3]:text-sm [&_h3]:font-semibold [&_h3]:text-text-primary [&_p]:my-2 [&_ul]:my-2 [&_ul]:pl-5 [&_ol]:my-2 [&_ol]:pl-5 [&_li]:my-1 [&_a]:text-glow-green [&_a]:underline [&_code]:rounded-md [&_code]:bg-space-overlay/70 [&_code]:px-1.5 [&_code]:py-0.5 [&_code]:text-[0.85em] [&_pre]:my-3 [&_pre]:overflow-x-auto [&_pre]:rounded-lg [&_pre]:border [&_pre]:border-glass-border/70 [&_pre]:bg-space-deep/70 [&_pre]:p-3">
              <Streamdown>{updateInfo.body}</Streamdown>
            </div>
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
