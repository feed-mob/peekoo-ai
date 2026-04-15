import { relaunch } from "@tauri-apps/plugin-process";
import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";
import { normalizeReleaseNotes } from "@/lib/release-notes";

const UPDATE_CHECK_DELAY_MS = 3_000;

export interface AppUpdateInfo {
  version: string;
  body: string | null;
  releaseUrl: string;
  update: Update | null;
}

export interface AppUpdateInstallProgress {
  phase: "downloading" | "installing";
  downloadedBytes: number;
  totalBytes: number | null;
  percent: number | null;
}

function resolveReleaseUrl(update: Update): string {
  const rawUrl = update.rawJson?.html_url;
  if (typeof rawUrl === "string" && rawUrl.startsWith("http")) {
    return rawUrl;
  }

  const tag = update.version.startsWith("v") ? update.version : `v${update.version}`;
  return `https://github.com/feed-mob/peekoo-ai/releases/tag/${tag}`;
}

interface CheckForAppUpdatesOptions {
  forceInDev?: boolean;
}

function buildDevMockUpdateInfo(): AppUpdateInfo {
  const version = import.meta.env.VITE_FORCE_UPDATER_VERSION || "0.1.99-dev";
  const notes = normalizeReleaseNotes(
    import.meta.env.VITE_FORCE_UPDATER_NOTES ||
      "## What's Changed\n- Added markdown release notes rendering\n- Added installer progress bar with ETA\n- Added full changelog link",
  );

  return {
    version,
    body: notes,
    releaseUrl: `https://github.com/feed-mob/peekoo-ai/releases/tag/v${version.replace(/^v/, "")}`,
    update: null,
  };
}

export async function checkForAppUpdates(options?: CheckForAppUpdatesOptions): Promise<AppUpdateInfo | null> {
  if (!import.meta.env.PROD) {
    if (import.meta.env.DEV && options?.forceInDev) {
      return buildDevMockUpdateInfo();
    }
    return null;
  }

  await new Promise((resolve) => window.setTimeout(resolve, UPDATE_CHECK_DELAY_MS));

  try {
    const update = await check();

    if (!update) {
      return null;
    }

    return {
      version: update.version,
      body: normalizeReleaseNotes(update.body),
      releaseUrl: resolveReleaseUrl(update),
      update,
    };
  } catch (error) {
    console.warn("peekoo updater check failed", error);
    return null;
  }
}

export async function installAppUpdate(
  update: Update,
  onProgress?: (event: DownloadEvent) => void,
): Promise<void> {
  await update.downloadAndInstall(onProgress);
  await relaunch();
}
