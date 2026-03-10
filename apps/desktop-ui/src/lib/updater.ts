import { ask } from "@tauri-apps/plugin-dialog";
import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";

const UPDATE_CHECK_DELAY_MS = 3_000;

function formatUpdateMessage(version: string, notes?: string): string {
  const trimmedNotes = notes?.trim();

  if (!trimmedNotes) {
    return `Peekoo ${version} is available. Install it now and restart?`;
  }

  return `Peekoo ${version} is available.\n\n${trimmedNotes}\n\nInstall it now and restart?`;
}

export async function checkForAppUpdates(): Promise<void> {
  if (!import.meta.env.PROD) {
    return;
  }

  await new Promise((resolve) => window.setTimeout(resolve, UPDATE_CHECK_DELAY_MS));

  try {
    const update = await check();

    if (!update) {
      return;
    }

    const shouldInstall = await ask(formatUpdateMessage(update.version, update.body), {
      title: "Update Available",
      kind: "info",
      okLabel: "Install and Restart",
      cancelLabel: "Later",
    });

    if (!shouldInstall) {
      return;
    }

    await update.downloadAndInstall();
    await relaunch();
  } catch (error) {
    console.warn("peekoo updater check failed", error);
  }
}
