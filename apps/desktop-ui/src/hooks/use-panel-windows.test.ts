import { describe, expect, test } from "bun:test";
import { calculatePanelPosition } from "./use-panel-windows";

describe("calculatePanelPosition", () => {
  test("places the panel to the right when there is enough room", () => {
    const position = calculatePanelPosition({
      spriteX: 100,
      spriteY: 80,
      spriteWidth: 200,
      panelWidth: 320,
      panelHeight: 400,
      workArea: {
        position: { x: 0, y: 0 },
        size: { width: 1440, height: 900 },
      },
    });

    expect(position).toEqual({ x: 320, y: 80 });
  });

  test("falls back to the left when the right side would be off-screen", () => {
    const position = calculatePanelPosition({
      spriteX: 1080,
      spriteY: 120,
      spriteWidth: 200,
      panelWidth: 320,
      panelHeight: 400,
      workArea: {
        position: { x: 0, y: 0 },
        size: { width: 1440, height: 900 },
      },
    });

    expect(position).toEqual({ x: 740, y: 120 });
  });

  test("clamps within the work area when neither side fully fits", () => {
    const position = calculatePanelPosition({
      spriteX: 420,
      spriteY: 760,
      spriteWidth: 180,
      panelWidth: 460,
      panelHeight: 300,
      workArea: {
        position: { x: 0, y: 24 },
        size: { width: 900, height: 700 },
      },
    });

    expect(position).toEqual({ x: 424, y: 408 });
  });
});
