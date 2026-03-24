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
      height: 346,
      extraLeft: 40,
      positionCompensationTop: 0,
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
      height: 286,
      extraLeft: 0,
      positionCompensationTop: 0,
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
      width: 320,
      height: 570,
      extraLeft: 60,
      positionCompensationTop: 224,
      extraTop: 224,
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
      height: 426,
      extraLeft: 40,
      positionCompensationTop: 80,
      extraTop: 80,
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
      paddingTop: 36,
      paddingBottom: 0,
      paddingLeft: 40,
      paddingRight: 40,
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
      paddingTop: 260,
      paddingBottom: 0,
      paddingLeft: 60,
      paddingRight: 60,
    });
  });

  test("centers sprite between compact bubble and input tray", () => {
    expect(
      getSpriteStagePadding({
        menuOpen: false,
        bubbleOpen: false,
        peekBadgeItemCount: 0,
        peekBadgeExpanded: false,
        miniChatOpen: true,
        miniChatBubbleOpen: true,
        miniChatBubbleExpanded: false,
      }),
    ).toEqual({
      paddingTop: 116,
      paddingBottom: 0,
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
      paddingTop: 36,
      paddingBottom: 0,
      paddingLeft: 0,
      paddingRight: 0,
    });
  });
});
