import { ask } from "@tauri-apps/plugin-dialog";
import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";
import i18next from "i18next";

const UPDATE_CHECK_DELAY_MS = 3_000;

function formatUpdateMessage(version: string, notes?: string): string {
  const trimmedNotes = notes?.trim();
  const t = i18next.t.bind(i18next);

  if (!trimmedNotes) {
    return t("updater.message", { version });
  }

  return t("updater.messageWithNotes", { version, notes: trimmedNotes });
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
      title: i18next.t("updater.title"),
      kind: "info",
      okLabel: i18next.t("updater.installButton"),
      cancelLabel: i18next.t("updater.later"),
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
