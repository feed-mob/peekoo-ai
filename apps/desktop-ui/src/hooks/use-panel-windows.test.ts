import { describe, expect, test } from "bun:test";
import { calculatePanelPosition } from "./use-panel-windows";

describe("calculatePanelPosition", () => {
  test("uses the sprite position when no work area is available", () => {
    const position = calculatePanelPosition({
      spriteX: 100,
      spriteY: 80,
      spriteWidth: 200,
      panelWidth: 320,
      panelHeight: 400,
    });

    expect(position).toEqual({ x: 320, y: 80 });
  });

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

  test("allows the panel to sit exactly on the right edge boundary", () => {
    const position = calculatePanelPosition({
      spriteX: 864,
      spriteY: 120,
      spriteWidth: 220,
      panelWidth: 320,
      panelHeight: 400,
      workArea: {
        position: { x: 0, y: 0 },
        size: { width: 1440, height: 900 },
      },
    });

    expect(position).toEqual({ x: 1104, y: 120 });
  });

  test("clamps to the top margin when the sprite is above the work area", () => {
    const position = calculatePanelPosition({
      spriteX: 220,
      spriteY: -20,
      spriteWidth: 180,
      panelWidth: 320,
      panelHeight: 240,
      workArea: {
        position: { x: 0, y: 24 },
        size: { width: 1440, height: 900 },
      },
    });

    expect(position).toEqual({ x: 420, y: 40 });
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

  test("clamps correctly for monitors with negative origins", () => {
    const position = calculatePanelPosition({
      spriteX: -520,
      spriteY: 640,
      spriteWidth: 160,
      panelWidth: 320,
      panelHeight: 240,
      workArea: {
        position: { x: -1440, y: 0 },
        size: { width: 1440, height: 900 },
      },
    });

    expect(position).toEqual({ x: -340, y: 640 });
  });

  test("pins the panel to the minimum visible origin when it is larger than the work area", () => {
    const position = calculatePanelPosition({
      spriteX: 120,
      spriteY: 80,
      spriteWidth: 120,
      panelWidth: 460,
      panelHeight: 320,
      workArea: {
        position: { x: 0, y: 24 },
        size: { width: 420, height: 300 },
      },
    });

    expect(position).toEqual({ x: 16, y: 40 });
  });
});
