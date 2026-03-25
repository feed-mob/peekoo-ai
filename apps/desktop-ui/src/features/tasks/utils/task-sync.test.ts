import { describe, expect, test } from "bun:test";
import { formatSyncStatus } from "./task-sync";

describe("formatSyncStatus", () => {
  test("shows syncing message while refresh is in flight", () => {
    expect(formatSyncStatus(true, null, Date.now())).toBe("Syncing…");
  });

  test("shows waiting state before first sync completes", () => {
    expect(formatSyncStatus(false, null, null)).toBe("Waiting for sync");
  });

  test("shows a just now label for recent syncs", () => {
    expect(formatSyncStatus(false, Date.now() - 1_000, Date.now())).toBe("Updated just now");
  });
});
