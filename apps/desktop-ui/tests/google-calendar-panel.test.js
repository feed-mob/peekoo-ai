import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const html = readFileSync(
  resolve(import.meta.dir, "../../../plugins/google-calendar/ui/panel.html"),
  "utf8",
);

describe("google calendar panel", () => {
  test("uses a tab switch for agenda views", () => {
    expect(html).toContain('id="tabUpcoming"');
    expect(html).toContain('id="tabDaily"');
    expect(html).toContain('id="tabWeekly"');
    expect(html).toContain('id="agendaList"');
  });
});
