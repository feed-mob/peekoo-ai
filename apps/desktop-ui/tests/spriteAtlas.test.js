import { describe, expect, test } from "bun:test";
import {
  buildBoundaries,
  buildAtlasGrid,
  getFrameRect,
  trimFrameTop,
} from "../src/components/sprite/spriteAtlas";

describe("sprite atlas boundaries", () => {
  test("builds contiguous boundaries for non-divisible dimensions", () => {
    const boundaries = buildBoundaries(1920, 7);

    expect(boundaries[0]).toBe(0);
    expect(boundaries.at(-1)).toBe(1920);
    expect(boundaries).toEqual([0, 274, 549, 823, 1097, 1371, 1646, 1920]);
  });

  test("creates in-bounds frame rects for 2208x1920 over 8x7", () => {
    const grid = buildAtlasGrid(2208, 1920, 8, 7);

    for (let row = 0; row < 7; row += 1) {
      for (let col = 0; col < 8; col += 1) {
        const frame = getFrameRect(grid, row, col);

        expect(frame).not.toBeNull();
        expect(frame.sx).toBeGreaterThanOrEqual(0);
        expect(frame.sy).toBeGreaterThanOrEqual(0);
        expect(frame.sw).toBeGreaterThan(0);
        expect(frame.sh).toBeGreaterThan(0);
        expect(frame.sx + frame.sw).toBeLessThanOrEqual(2208);
        expect(frame.sy + frame.sh).toBeLessThanOrEqual(1920);
      }
    }
  });

  test("sleepy row starts at computed boundary instead of row*274", () => {
    const grid = buildAtlasGrid(2208, 1920, 8, 7);
    const frame = getFrameRect(grid, 5, 0);

    expect(frame.sy).toBe(1371);
    expect(frame.sy).not.toBe(5 * 274);
  });

  test("trims top pixels while preserving the same bottom edge", () => {
    const original = { sx: 10, sy: 100, sw: 50, sh: 200 };
    const trimmed = trimFrameTop(original, 9);

    expect(trimmed.sy).toBe(109);
    expect(trimmed.sh).toBe(191);
    expect(trimmed.sy + trimmed.sh).toBe(original.sy + original.sh);
  });

  test("clamps top trim to avoid non-positive frame height", () => {
    const original = { sx: 10, sy: 100, sw: 50, sh: 6 };
    const trimmed = trimFrameTop(original, 10);

    expect(trimmed.sy).toBe(105);
    expect(trimmed.sh).toBe(1);
  });
});
