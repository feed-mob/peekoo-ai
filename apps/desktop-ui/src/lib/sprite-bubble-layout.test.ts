import { describe, expect, test } from "bun:test";
import {
  getSpriteStagePadding,
  getSpriteWindowSize,
  MINI_CHAT_OPEN_WIDTH,
  SPRITE_WINDOW_SIZE,
} from "./sprite-bubble-layout";

describe("getSpriteWindowSize", () => {
  test("widens sprite window when mini chat is open", () => {
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
      width: MINI_CHAT_OPEN_WIDTH,
      height: 320,
      extraLeft: 20,
      extraTop: 0,
    });
  });

  test("shrinks sprite window back when mini chat closes", () => {
    expect(
      getSpriteWindowSize({
        menuOpen: false,
        bubbleOpen: false,
        peekBadgeItemCount: 0,
        peekBadgeExpanded: false,
        miniChatOpen: false,
      }),
    ).toEqual({
      width: SPRITE_WINDOW_SIZE.width,
      height: SPRITE_WINDOW_SIZE.height,
      extraLeft: 0,
      extraTop: 0,
    });
  });

  test("widens further for expanded mini chat replies", () => {
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

  test("uses widened width for mini chat with reply bubble", () => {
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
      width: MINI_CHAT_OPEN_WIDTH,
      height: 390,
      extraLeft: 20,
      extraTop: 120,
    });
  });
});

describe("getSpriteStagePadding", () => {
  test("centers sprite with equal padding when mini chat is open", () => {
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
      paddingLeft: 20,
      paddingRight: 20,
    });
  });

  test("adds more padding for expanded reading mode", () => {
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

  test("uses no extra padding when mini chat is closed", () => {
    expect(
      getSpriteStagePadding({
        menuOpen: false,
        bubbleOpen: false,
        peekBadgeItemCount: 0,
        peekBadgeExpanded: false,
        miniChatOpen: false,
      }),
    ).toEqual({
      paddingTop: 12,
      paddingBottom: 12,
      paddingLeft: 0,
      paddingRight: 0,
    });
  });
});
