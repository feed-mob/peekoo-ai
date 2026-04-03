import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const panelJs = readFileSync(join(import.meta.dir, "panel.js"), "utf8");

describe("google calendar panel js", () => {
  test("uses camelCase for create_task invoke", () => {
    expect(panelJs).toContain("scheduledStartAt: schedule.startAt");
    expect(panelJs).toContain("scheduledEndAt: schedule.endAt");
    expect(panelJs).not.toContain("scheduled_start_at: schedule");
  });

  test("uses camelCase for update_task invoke", () => {
    expect(panelJs).toContain("scheduledStartAt: schedule.startAt");
    expect(panelJs).toContain("scheduledEndAt: schedule.endAt");
    expect(panelJs).not.toContain("scheduled_start_at: schedule");
  });
});
