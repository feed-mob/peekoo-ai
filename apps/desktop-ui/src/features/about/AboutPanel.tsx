import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { useAboutPanel } from "./useAboutPanel";

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
  const { snapshot, loading, checking, installing, error, refresh, installUpdate } = useAboutPanel();

  if (loading && !snapshot) {
    return (
      <div className="flex h-36 items-center justify-center text-sm text-text-muted">
        Loading about info...
      </div>
    );
  }

  if (!snapshot) {
    return (
      <div className="space-y-4">
        <div className="rounded-xl border border-danger/40 bg-danger/10 px-4 py-3 text-sm text-danger">
          Failed to load app details{error ? `: ${error}` : "."}
        </div>
        <Button variant="glass" onClick={() => void refresh()} disabled={checking}>
          {checking ? "Checking..." : "Try Again"}
        </Button>
      </div>
    );
  }

  const publishedLabel = formatReleaseDate(snapshot.releaseDate);

  return (
    <div className="space-y-5">
      <div className="rounded-2xl border border-glass-border bg-space-overlay/55 px-4 py-4 shadow-lg shadow-black/10">
        <div className="flex items-center justify-between gap-3">
          <div>
            <p className="text-lg font-semibold text-text-primary">{snapshot.appName}</p>
            <p className="text-sm text-text-muted">Desktop companion and pet</p>
          </div>
          <Badge variant={snapshot.isUpdateAvailable ? "default" : "secondary"}>
            {snapshot.isUpdateAvailable ? "Update available" : "Up to date"}
          </Badge>
        </div>
      </div>

      <div className="space-y-3">
        <InfoRow label="Current" value={snapshot.currentVersion} />
        <InfoRow
          label="Latest"
          value={snapshot.availableVersion ?? snapshot.currentVersion}
        />
        {publishedLabel ? <InfoRow label="Published" value={publishedLabel} /> : null}
      </div>

      {!import.meta.env.PROD ? (
        <div className="rounded-xl border border-glass-border/70 bg-space-overlay/40 px-4 py-3 text-sm text-text-muted">
          Update checks are disabled in development builds.
        </div>
      ) : null}

      {snapshot.releaseNotes ? (
        <div className="space-y-2 rounded-xl border border-glass-border/70 bg-space-overlay/40 px-4 py-3">
          <p className="text-xs uppercase tracking-[0.18em] text-text-muted">Release notes</p>
          <p className="whitespace-pre-wrap text-sm leading-6 text-text-primary">{snapshot.releaseNotes}</p>
        </div>
      ) : null}

      {error ? (
        <div className="rounded-xl border border-danger/40 bg-danger/10 px-4 py-3 text-sm text-danger">
          {error}
        </div>
      ) : null}

      <div className="flex gap-3">
        <Button variant="glass" onClick={() => void refresh()} disabled={checking || installing}>
          {checking ? "Checking..." : "Check for Updates"}
        </Button>
        {snapshot.isUpdateAvailable ? (
          <Button variant="success" onClick={() => void installUpdate()} disabled={installing || checking}>
            {installing ? "Installing..." : "Install and Restart"}
          </Button>
        ) : null}
      </div>
    </div>
  );
}
