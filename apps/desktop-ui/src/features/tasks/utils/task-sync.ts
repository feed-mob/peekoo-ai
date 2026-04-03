import type { TFunction } from "i18next";

export function formatSyncStatus(
  isRefreshing: boolean,
  lastSyncedAt: number | null,
  now = Date.now(),
  t: TFunction
): string {
  if (isRefreshing) {
    return t("tasks.sync.syncing");
  }

  if (!lastSyncedAt) {
    return t("tasks.sync.waiting");
  }

  const diffSeconds = Math.max(0, Math.floor((now - lastSyncedAt) / 1000));

  if (diffSeconds < 5) {
    return t("tasks.sync.updatedJustNow");
  }

  if (diffSeconds < 60) {
    return t("tasks.sync.updatedSecondsAgo", { seconds: diffSeconds });
  }

  const diffMinutes = Math.floor(diffSeconds / 60);
  return t("tasks.sync.updatedMinutesAgo", { minutes: diffMinutes });
}
