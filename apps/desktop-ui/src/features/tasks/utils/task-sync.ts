export function formatSyncStatus(
  isRefreshing: boolean,
  lastSyncedAt: number | null,
  now = Date.now()
): string {
  if (isRefreshing) {
    return "Syncing…";
  }

  if (!lastSyncedAt) {
    return "Waiting for sync";
  }

  const diffSeconds = Math.max(0, Math.floor((now - lastSyncedAt) / 1000));

  if (diffSeconds < 5) {
    return "Updated just now";
  }

  if (diffSeconds < 60) {
    return `Updated ${diffSeconds}s ago`;
  }

  const diffMinutes = Math.floor(diffSeconds / 60);
  return `Updated ${diffMinutes}m ago`;
}
