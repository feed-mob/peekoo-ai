import type { TFunction } from "i18next";

export function formatSyncStatus(
  isRefreshing: boolean,
  lastSyncedAt: number | null,
  now = Date.now(),
  t?: TFunction
): string {
  if (isRefreshing) {
    return t ? t("tasks.sync.syncing") : "Syncing…";
  }

  if (!lastSyncedAt) {
    return t ? t("tasks.sync.waiting") : "Waiting for sync";
  }

  const diffSeconds = Math.max(0, Math.floor((now - lastSyncedAt) / 1000));

  if (diffSeconds < 5) {
    return t ? t("tasks.sync.updatedJustNow") : "Updated just now";
  }

  if (diffSeconds < 60) {
    return t ? t("tasks.sync.updatedSecondsAgo", { seconds: diffSeconds }) : `Updated ${diffSeconds}s ago`;
  }

  const diffMinutes = Math.floor(diffSeconds / 60);
  return t ? t("tasks.sync.updatedMinutesAgo", { minutes: diffMinutes }) : `Updated ${diffMinutes}m ago`;
}
