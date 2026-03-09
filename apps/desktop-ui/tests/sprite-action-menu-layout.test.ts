import { describe, expect, test } from "bun:test";
import { getSpriteActionMenuItems } from "../src/components/sprite/spriteActionMenuLayout";

describe("getSpriteActionMenuItems", () => {
  test("returns all four built-in items in order: Chat, Tasks, Pomodoro, Plugins", () => {
    const items = getSpriteActionMenuItems([]);
    const labels = items.map((item) => item.label);

    expect(labels).toEqual([
      "panel-chat",
      "panel-tasks",
      "panel-pomodoro",
      "panel-plugins",
    ]);
  });

  test("places all items on the same horizontal row below the sprite center", () => {
    const items = getSpriteActionMenuItems([]);

    const yValues = items.map((item) => item.y);
    const allSameY = yValues.every((y) => y === yValues[0]);

    expect(allSameY).toBe(true);
    expect(yValues[0]).toBeGreaterThan(0);
  });

  test("items are evenly spaced left-to-right, centered on x=0", () => {
    const items = getSpriteActionMenuItems([]);
    const xs = items.map((item) => item.x);

    // centered: first + last should sum to ~0
    expect(xs[0] + xs[xs.length - 1]).toBeCloseTo(0, 5);

    // even spacing between consecutive items
    const gaps = xs.slice(1).map((x, i) => x - xs[i]);
    const firstGap = gaps[0];

    for (const gap of gaps) {
      expect(gap).toBeCloseTo(firstGap, 5);
    }
  });

  test("appends dynamic plugin panels to the row", () => {
    const pluginPanels = [
      { label: "plugin-panel-health" as const, title: "Health", pluginKey: "health-reminders" },
    ];

    const items = getSpriteActionMenuItems(pluginPanels as any);

    expect(items).toHaveLength(5);
    expect(items[4].label).toBe("plugin-panel-health");
    expect(items[4].y).toBe(items[0].y);
  });
});
