import { describe, expect, test } from "bun:test";
import { formatSyncStatus } from "./task-sync";

const mockT = ((key: string) => key) as import("i18next").TFunction;

describe("formatSyncStatus", () => {
  test("shows syncing message while refresh is in flight", () => {
    expect(formatSyncStatus(true, null, Date.now(), mockT)).toBe("tasks.sync.syncing");
  });

  test("shows waiting state before first sync completes", () => {
    expect(formatSyncStatus(false, null, undefined, mockT)).toBe("tasks.sync.waiting");
  });

  test("shows a just now label for recent syncs", () => {
    expect(formatSyncStatus(false, Date.now() - 1_000, Date.now(), mockT)).toBe("tasks.sync.updatedJustNow");
  });
});
