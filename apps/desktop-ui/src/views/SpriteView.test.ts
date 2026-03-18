import { describe, expect, mock, test } from "bun:test";
import { openAboutPanelFromTray, openSettingsPanelFromTray } from "./SpriteView";

describe("openSettingsPanelFromTray", () => {
  test("opens or focuses the settings panel", async () => {
    const openPanel = mock(async () => {});

    await openSettingsPanelFromTray(openPanel);

    expect(openPanel).toHaveBeenCalledTimes(1);
    expect(openPanel).toHaveBeenCalledWith("panel-settings");
  });
});

describe("openAboutPanelFromTray", () => {
  test("opens or focuses the about panel", async () => {
    const openPanel = mock(async () => {});

    await openAboutPanelFromTray(openPanel);

    expect(openPanel).toHaveBeenCalledTimes(1);
    expect(openPanel).toHaveBeenCalledWith("panel-about");
  });
});
