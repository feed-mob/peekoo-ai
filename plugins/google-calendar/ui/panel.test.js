import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const panelCss = readFileSync(join(import.meta.dir, "panel.css"), "utf8");

describe("google calendar panel styles", () => {
  test("keeps settings panel hidden when collapsed", () => {
    expect(panelCss).toContain(".settings-panel.hidden {");
    expect(panelCss).toContain("display: none;");
  });
});
