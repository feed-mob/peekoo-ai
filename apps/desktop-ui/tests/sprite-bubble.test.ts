import { describe, expect, test } from "bun:test";
import {
  BUBBLE_EXTRA_HEIGHT,
  MENU_EXTRA_HEIGHT,
  PEEK_BADGE_HEIGHT,
  PEEK_BADGE_PADDING,
  SPRITE_BUBBLE_WINDOW_SIZE,
  SPRITE_MENU_WINDOW_SIZE,
  SPRITE_WIDTH,
  SPRITE_WINDOW_SIZE,
  getSpriteWindowSize,
  peekBadgeExtraHeight,
} from "../src/lib/sprite-bubble-layout";
import { SpriteBubblePayloadSchema } from "../src/types/sprite-bubble";
import { PeekBadgeItemSchema } from "../src/types/peek-badge";

const NO_BADGE = { peekBadgeItemCount: 0, peekBadgeExpanded: false };

describe("getSpriteWindowSize", () => {
  test("returns sprite size when idle", () => {
    const size = getSpriteWindowSize({ menuOpen: false, bubbleOpen: false, ...NO_BADGE });
    expect(size.width).toBe(SPRITE_WINDOW_SIZE.width);
    expect(size.height).toBe(286);
    expect(size.extraTop).toBe(0);
  });

  test("expands height upward when bubble is visible", () => {
    const size = getSpriteWindowSize({ menuOpen: false, bubbleOpen: true, ...NO_BADGE });
    expect(size.width).toBe(SPRITE_WIDTH);
    expect(size.height).toBe(406);
    expect(size.extraTop).toBe(BUBBLE_EXTRA_HEIGHT);
  });

  test("expands height downward when menu is open", () => {
    const size = getSpriteWindowSize({ menuOpen: true, bubbleOpen: false, ...NO_BADGE });
    expect(size.width).toBe(SPRITE_WIDTH);
    expect(size.height).toBe(346);
    expect(size.extraTop).toBe(60);
  });

  test("takes max height when both bubble and menu are active", () => {
    const size = getSpriteWindowSize({ menuOpen: true, bubbleOpen: true, ...NO_BADGE });
    expect(size.width).toBe(SPRITE_WIDTH);
    expect(size.height).toBe(406);
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

  test("expands upward for peek badge when collapsed", () => {
    const size = getSpriteWindowSize({
      menuOpen: false,
      bubbleOpen: false,
      peekBadgeItemCount: 3,
      peekBadgeExpanded: false,
    });
    const expectedExtra = PEEK_BADGE_HEIGHT + PEEK_BADGE_PADDING;
    expect(size.height).toBe(286 + expectedExtra);
    expect(size.extraTop).toBe(0);
  });

  test("hides badge height when bubble is open", () => {
    const size = getSpriteWindowSize({
      menuOpen: false,
      bubbleOpen: true,
      peekBadgeItemCount: 3,
      peekBadgeExpanded: false,
    });
    expect(size.height).toBe(406 + PEEK_BADGE_HEIGHT + PEEK_BADGE_PADDING);
    expect(size.extraTop).toBe(BUBBLE_EXTRA_HEIGHT);
  });

  test("hides badge height when menu is open", () => {
    const size = getSpriteWindowSize({
      menuOpen: true,
      bubbleOpen: false,
      peekBadgeItemCount: 3,
      peekBadgeExpanded: false,
    });
    expect(size.height).toBe(346);
    expect(size.extraTop).toBe(60);
  });
});

describe("peekBadgeExtraHeight", () => {
  test("returns zero when no items", () => {
    expect(peekBadgeExtraHeight(0, false)).toBe(0);
    expect(peekBadgeExtraHeight(0, true)).toBe(0);
  });

  test("returns collapsed height for any item count", () => {
    const height = peekBadgeExtraHeight(3, false);
    expect(height).toBe(PEEK_BADGE_HEIGHT + PEEK_BADGE_PADDING);
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

describe("PeekBadgeItemSchema", () => {
  test("parses a valid badge item with all fields", () => {
    const parsed = PeekBadgeItemSchema.safeParse({
      label: "Eye Rest",
      value: "~4 min",
      icon: "eye",
      countdown_secs: 240,
    });
    expect(parsed.success).toBe(true);
  });

  test("parses a badge item without optional fields", () => {
    const parsed = PeekBadgeItemSchema.safeParse({
      label: "Water",
      value: "~20 min",
    });
    expect(parsed.success).toBe(true);
  });

  test("rejects missing label", () => {
    const parsed = PeekBadgeItemSchema.safeParse({ value: "~4 min" });
    expect(parsed.success).toBe(false);
  });
});
