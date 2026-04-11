import type { InstallPhase } from "@/lib/update-install-progress";
import { useTranslation } from "react-i18next";

interface InstallProgressCardProps {
  installPhase: InstallPhase;
  downloadedBytes: number;
  totalBytes: number | null;
  progressPercent: number | null;
  etaSeconds: number | null;
  className?: string;
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

export function InstallProgressCard({
  installPhase,
  downloadedBytes,
  totalBytes,
  progressPercent,
  etaSeconds,
  className,
}: InstallProgressCardProps) {
  const { t } = useTranslation();

  return (
    <div className={className ?? "rounded-lg border border-glass-border/70 bg-space-overlay/45 px-3 py-3"}>
      <div className="mb-2 flex items-center justify-between gap-2 text-xs text-text-muted">
        <span>
          {installPhase === "installing" ? t("updater.installingFiles") : t("updater.downloading")}
        </span>
        <span>{progressPercent !== null ? `${progressPercent}%` : t("updater.calculating")}</span>
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
            <p className="text-xs text-text-muted">{t("updater.eta", { eta: formatDuration(etaSeconds) })}</p>
          ) : null}
        </div>
      ) : (
        <p className="mt-2 text-xs text-text-muted">{t("updater.restartSoon")}</p>
      )}
    </div>
  );
}
