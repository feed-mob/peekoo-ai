import { describe, expect, test } from "bun:test";
import { getSpriteCanvasSize } from "../src/components/sprite/spriteCanvasSize";

describe("getSpriteCanvasSize", () => {
  test("returns matching logical and backing sizes at dpr 1", () => {
    expect(getSpriteCanvasSize({
      nominalFrameWidth: 276,
      nominalFrameHeight: 274,
      scale: 0.4,
      devicePixelRatio: 1,
    })).toEqual({
      displayWidth: 110,
      displayHeight: 110,
      canvasWidth: 110,
      canvasHeight: 110,
      devicePixelRatio: 1,
    });
  });

  test("keeps logical size stable while scaling backing canvas for fractional dpr", () => {
    expect(getSpriteCanvasSize({
      nominalFrameWidth: 276,
      nominalFrameHeight: 274,
      scale: 0.4,
      devicePixelRatio: 1.25,
    })).toEqual({
      displayWidth: 110,
      displayHeight: 110,
      canvasWidth: 138,
      canvasHeight: 138,
      devicePixelRatio: 1.25,
    });
  });

  test("clamps very small scaled sprites to at least one pixel", () => {
    expect(getSpriteCanvasSize({
      nominalFrameWidth: 10,
      nominalFrameHeight: 8,
      scale: 0.01,
      devicePixelRatio: 1.5,
    })).toEqual({
      displayWidth: 1,
      displayHeight: 1,
      canvasWidth: 2,
      canvasHeight: 2,
      devicePixelRatio: 1.5,
    });
  });
});
