import { describe, expect, test } from "bun:test";
import {
  BUBBLE_EXTRA_HEIGHT,
  MENU_EXTRA_HEIGHT,
  SPRITE_BUBBLE_WINDOW_SIZE,
  SPRITE_MENU_WINDOW_SIZE,
  SPRITE_WIDTH,
  SPRITE_WINDOW_SIZE,
  getSpriteWindowSize,
} from "../src/lib/sprite-bubble-layout";
import { SpriteBubblePayloadSchema } from "../src/types/sprite-bubble";

describe("getSpriteWindowSize", () => {
  test("returns sprite size when idle", () => {
    const size = getSpriteWindowSize({ menuOpen: false, bubbleOpen: false });
    expect(size.width).toBe(SPRITE_WINDOW_SIZE.width);
    expect(size.height).toBe(SPRITE_WINDOW_SIZE.height);
    expect(size.extraTop).toBe(0);
  });

  test("expands height upward when bubble is visible", () => {
    const size = getSpriteWindowSize({ menuOpen: false, bubbleOpen: true });
    expect(size.width).toBe(SPRITE_WIDTH);
    expect(size.height).toBe(SPRITE_BUBBLE_WINDOW_SIZE.height);
    expect(size.extraTop).toBe(BUBBLE_EXTRA_HEIGHT);
  });

  test("expands height downward when menu is open", () => {
    const size = getSpriteWindowSize({ menuOpen: true, bubbleOpen: false });
    expect(size.width).toBe(SPRITE_WIDTH);
    expect(size.height).toBe(SPRITE_MENU_WINDOW_SIZE.height);
    expect(size.extraTop).toBe(0);
  });

  test("takes max height when both bubble and menu are active", () => {
    const size = getSpriteWindowSize({ menuOpen: true, bubbleOpen: true });
    expect(size.width).toBe(SPRITE_WIDTH);
    expect(size.height).toBe(
      Math.max(SPRITE_MENU_WINDOW_SIZE.height, SPRITE_BUBBLE_WINDOW_SIZE.height)
    );
    expect(size.extraTop).toBe(BUBBLE_EXTRA_HEIGHT);
  });

  test("bubble window height includes extra top space", () => {
    expect(SPRITE_BUBBLE_WINDOW_SIZE.height).toBe(
      SPRITE_WINDOW_SIZE.height + BUBBLE_EXTRA_HEIGHT
    );
  });

  test("menu window height includes extra bottom space", () => {
    expect(SPRITE_MENU_WINDOW_SIZE.height).toBe(
      SPRITE_WINDOW_SIZE.height + MENU_EXTRA_HEIGHT
    );
  });
});

describe("SpriteBubblePayloadSchema", () => {
  test("parses a valid notification payload", () => {
    const parsed = SpriteBubblePayloadSchema.safeParse({
      title: "Health Reminder",
      body: "Time to drink water.",
    });
    expect(parsed.success).toBe(true);
  });

  test("rejects missing body", () => {
    const parsed = SpriteBubblePayloadSchema.safeParse({ title: "Hi" });
    expect(parsed.success).toBe(false);
  });
});
