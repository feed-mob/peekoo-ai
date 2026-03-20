import { describe, expect, test } from "bun:test";
import {
  getSpriteStagePadding,
  getSpriteWindowSize,
  SPRITE_WINDOW_SIZE,
} from "./sprite-bubble-layout";

describe("getSpriteWindowSize", () => {
  test("grows upward and downward when mini chat is open with a reply bubble", () => {
    expect(
      getSpriteWindowSize({
        menuOpen: false,
        bubbleOpen: false,
        peekBadgeItemCount: 0,
        peekBadgeExpanded: false,
        miniChatOpen: true,
        miniChatBubbleOpen: true,
      }),
    ).toEqual({
      width: SPRITE_WINDOW_SIZE.width,
      height: 390,
      extraLeft: 0,
      extraTop: 120,
    });
  });

  test("grows only downward when mini chat is open without a reply bubble", () => {
    expect(
      getSpriteWindowSize({
        menuOpen: false,
        bubbleOpen: false,
        peekBadgeItemCount: 0,
        peekBadgeExpanded: false,
        miniChatOpen: true,
        miniChatBubbleOpen: false,
      }),
    ).toEqual({
      width: SPRITE_WINDOW_SIZE.width,
      height: 320,
      extraLeft: 0,
      extraTop: 0,
    });
  });

  test("grows further upward for expanded mini chat replies", () => {
    expect(
      getSpriteWindowSize({
        menuOpen: false,
        bubbleOpen: false,
        peekBadgeItemCount: 0,
        peekBadgeExpanded: false,
        miniChatOpen: true,
        miniChatBubbleOpen: true,
        miniChatBubbleExpanded: true,
      }),
    ).toEqual({
      width: 280,
      height: 470,
      extraLeft: 40,
      extraTop: 200,
    });
  });
});

describe("getSpriteStagePadding", () => {
  test("keeps the sprite pushed below expanded reading mode", () => {
    expect(
      getSpriteStagePadding({
        menuOpen: false,
        bubbleOpen: false,
        peekBadgeItemCount: 0,
        peekBadgeExpanded: false,
        miniChatOpen: true,
        miniChatBubbleOpen: true,
        miniChatBubbleExpanded: true,
      }),
    ).toEqual({
      paddingTop: 160,
      paddingBottom: 86,
      paddingLeft: 40,
      paddingRight: 40,
    });
  });

  test("keeps compact mini chat centered with minimal padding", () => {
    expect(
      getSpriteStagePadding({
        menuOpen: false,
        bubbleOpen: false,
        peekBadgeItemCount: 0,
        peekBadgeExpanded: false,
        miniChatOpen: true,
        miniChatBubbleOpen: false,
        miniChatBubbleExpanded: false,
      }),
    ).toEqual({
      paddingTop: 12,
      paddingBottom: 86,
      paddingLeft: 0,
      paddingRight: 0,
    });
  });
});
