import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { InstallProgressCard } from "@/components/update/InstallProgressCard";
import { ReleaseNotesMarkdown } from "@/components/update/ReleaseNotesMarkdown";
import { normalizeReleaseNotes } from "@/lib/release-notes";
import { InstallProgressCard } from "@/components/update/InstallProgressCard";
import { ReleaseNotesMarkdown } from "@/components/update/ReleaseNotesMarkdown";
import { invoke } from "@tauri-apps/api/core";
import { useAboutPanel } from "./useAboutPanel";
import { useTranslation } from "react-i18next";

function formatReleaseDate(value: string | null): string | null {
  if (!value) {
    return null;
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return null;
  }

  return new Intl.DateTimeFormat(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  }).format(date);
}

function InfoRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-3 rounded-xl border border-glass-border/70 bg-space-overlay/40 px-3 py-2">
      <span className="text-xs uppercase tracking-[0.18em] text-text-muted">{label}</span>
      <span className="text-sm font-medium text-text-primary">{value}</span>
    </div>
  );
}

export function AboutPanel() {
  const { t } = useTranslation();
  const {
    snapshot,
    loading,
    checking,
    installing,
    installPhase,
    downloadedBytes,
    totalBytes,
    progressPercent,
    etaSeconds,
    error,
    refresh,
    installUpdate,
  } = useAboutPanel();

  if (loading && !snapshot) {
    return (
      <div className="flex h-36 items-center justify-center text-sm text-text-muted">
        {t("about.loading")}
      </div>
    );
  }

  if (!snapshot) {
    return (
      <div className="space-y-4">
        <div className="rounded-xl border border-danger/40 bg-danger/10 px-4 py-3 text-sm text-danger">
          {t("about.failedLoad")}
          {error ? `: ${error}` : "."}
        </div>
        <Button variant="glass" onClick={() => void refresh()} disabled={checking}>
          {checking ? t("about.checking") : t("about.tryAgain")}
        </Button>
      </div>
    );
  }

  const publishedLabel = formatReleaseDate(snapshot.releaseDate);
  const normalizedReleaseNotes = normalizeReleaseNotes(snapshot.releaseNotes);

  return (
    <div className="space-y-5">
      <div className="rounded-2xl border border-glass-border bg-space-overlay/55 px-4 py-4 shadow-lg shadow-black/10">
        <div className="flex items-center justify-between gap-3">
          <div>
            <p className="text-lg font-semibold text-text-primary">{snapshot.appName}</p>
            <p className="text-sm text-text-muted">{t("about.tagline")}</p>
          </div>
          <Badge variant={snapshot.isUpdateAvailable ? "default" : "secondary"}>
            {snapshot.isUpdateAvailable ? t("about.updateAvailable") : t("about.upToDate")}
          </Badge>
        </div>
      </div>

      <div className="space-y-3">
        <InfoRow label={t("about.current")} value={snapshot.currentVersion} />
        <InfoRow
          label={t("about.latest")}
          value={snapshot.availableVersion ?? snapshot.currentVersion}
        />
        {publishedLabel ? <InfoRow label={t("about.published")} value={publishedLabel} /> : null}
      </div>

      {!import.meta.env.PROD ? (
        <div className="rounded-xl border border-glass-border/70 bg-space-overlay/40 px-4 py-3 text-sm text-text-muted">
          {t("about.devUpdateDisabled")}
        </div>
      ) : null}

      {normalizedReleaseNotes ? (
        <div className="space-y-2 rounded-xl border border-glass-border/70 bg-space-overlay/40 px-4 py-3">
          <p className="text-xs uppercase tracking-[0.18em] text-text-muted">{t("about.releaseNotes")}</p>
          <ReleaseNotesMarkdown notes={normalizedReleaseNotes} />
        </div>
      ) : null}

      {error ? (
        <div className="rounded-xl border border-danger/40 bg-danger/10 px-4 py-3 text-sm text-danger">
          {error}
        </div>
      ) : null}

      {installing ? (
        <InstallProgressCard
          className="rounded-xl border border-glass-border/70 bg-space-overlay/40 px-4 py-3"
          installPhase={installPhase}
          downloadedBytes={downloadedBytes}
          totalBytes={totalBytes}
          progressPercent={progressPercent}
          etaSeconds={etaSeconds}
        />
      ) : null}

      <div className="flex flex-wrap gap-3">
        <Button variant="glass" onClick={() => void refresh()} disabled={checking || installing}>
          {checking ? t("about.checking") : t("about.checkForUpdates")}
        </Button>
        <Button variant="glass" onClick={() => void invoke("system_open_log_dir")}>
          {t("about.openLogs")}
        </Button>
        {snapshot.isUpdateAvailable ? (
          <Button variant="success" onClick={() => void installUpdate()} disabled={installing || checking}>
            {installing ? t("about.installing") : t("about.installAndRestart")}
          </Button>
        ) : null}
      </div>
    </div>
  );
}
