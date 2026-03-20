import { describe, expect, test } from "bun:test";
import {
  getLatestMiniChatMessage,
  getMiniChatReplyDisplayMode,
  getMiniChatVisibleMessage,
  mapSessionMessagesToMessages,
  type SessionMessageLike,
} from "./chat-session";

describe("mapSessionMessagesToMessages", () => {
  test("maps persisted chat history into UI messages", () => {
    const sessionMessages: SessionMessageLike[] = [
      { role: "user", text: "hello" },
      { role: "assistant", text: "hi there" },
    ];

    expect(mapSessionMessagesToMessages(sessionMessages)).toEqual([
      { id: "history-0", role: "user", text: "hello" },
      { id: "history-1", role: "pet", text: "hi there" },
    ]);
  });
});

describe("getLatestMiniChatMessage", () => {
  test("returns the latest assistant reply for the sprite mini chat", () => {
    expect(
      getLatestMiniChatMessage([
        { id: "1", role: "user", text: "Draft blog post" },
        { id: "2", role: "pet", text: "Sure, here is a quick outline." },
      ]),
    ).toEqual({
      id: "2",
      role: "pet",
      text: "Sure, here is a quick outline.",
    });
  });

  test("returns the latest error when the newest response failed", () => {
    expect(
      getLatestMiniChatMessage([
        { id: "1", role: "user", text: "Do the thing" },
        { id: "2", role: "error", text: "Error: backend unavailable" },
      ]),
    ).toEqual({
      id: "2",
      role: "error",
      text: "Error: backend unavailable",
    });
  });

  test("ignores trailing user messages when no reply has arrived yet", () => {
    expect(
      getLatestMiniChatMessage([
        { id: "1", role: "pet", text: "Last completed answer" },
        { id: "2", role: "user", text: "One more tweak" },
      ]),
    ).toEqual({
      id: "1",
      role: "pet",
      text: "Last completed answer",
    });
  });
});

describe("getMiniChatReplyDisplayMode", () => {
  test("keeps short replies in compact mode", () => {
    expect(
      getMiniChatReplyDisplayMode({
        id: "1",
        role: "pet",
        text: "A short answer.",
      }),
    ).toBe("compact");
  });

  test("expands long replies into reading mode", () => {
    expect(
      getMiniChatReplyDisplayMode({
        id: "2",
        role: "pet",
        text:
          "This is a much longer answer that should trigger the expanded reading card because it will be hard to read inside the compact bubble above the sprite.",
      }),
    ).toBe("expanded");
  });

  test("shows errors in expanded mode so they stay readable", () => {
    expect(
      getMiniChatReplyDisplayMode({
        id: "3",
        role: "error",
        text: "Error: failed to connect to local model after retrying several times.",
      }),
    ).toBe("expanded");
  });
});

describe("getMiniChatVisibleMessage", () => {
  test("hides stale history when mini chat has not sent anything yet", () => {
    expect(
      getMiniChatVisibleMessage({
        messages: [
          { id: "1", role: "user", text: "old prompt" },
          { id: "2", role: "pet", text: "old answer" },
        ],
        activeReplyId: null,
      }),
    ).toBeNull();
  });

  test("shows the reply tied to the current mini chat interaction", () => {
    expect(
      getMiniChatVisibleMessage({
        messages: [
          { id: "1", role: "user", text: "old prompt" },
          { id: "2", role: "pet", text: "old answer" },
          { id: "3", role: "user", text: "new prompt" },
          { id: "4", role: "pet", text: "new answer" },
        ],
        activeReplyId: "4",
      }),
    ).toEqual({ id: "4", role: "pet", text: "new answer" });
  });
});
