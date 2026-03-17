import { describe, expect, test } from "bun:test";
import {
  createBridgeController,
  type BridgeWrite,
} from "./peekoo-opencode-companion";

function textMessageEvent() {
  return {
    type: "message.part.updated",
    properties: { part: { type: "text" }, sessionID: "session-1" },
  };
}

describe("createBridgeController", () => {
  test("keeps the last known title after idle cleanup", () => {
    const writes: BridgeWrite[] = [];
    const idleTransitions = new Map<string, () => void>();

    const controller = createBridgeController({
      writeBridge: (state) => writes.push(state),
      scheduleIdle: (sessionId, callback) => {
        idleTransitions.set(sessionId, callback);
      },
      cancelIdle: (sessionId) => {
        idleTransitions.delete(sessionId);
      },
      now: () => 100,
    });

    controller.handleEvent({
      type: "session.created",
      properties: { sessionID: "session-1", title: "Fix repeated badge updates" },
    });
    controller.handleEvent({
      type: "session.idle",
      properties: { sessionID: "session-1" },
    });

    idleTransitions.get("session-1")?.();
    controller.handleEvent(textMessageEvent());

    expect(writes.map((write) => [write.status, write.session_title])).toEqual([
      ["working", "Fix repeated badge updates"],
      ["happy", "Fix repeated badge updates"],
      ["idle", ""],
      ["working", "Fix repeated badge updates"],
    ]);
  });

  test("emits all active sessions so the badge can rotate through them", () => {
    const writes: BridgeWrite[] = [];

    const controller = createBridgeController({
      writeBridge: (state) => writes.push(state),
      scheduleIdle: () => {},
      cancelIdle: () => {},
      now: (() => {
        let current = 200;
        return () => current++;
      })(),
    });

    controller.handleEvent({
      type: "session.created",
      properties: { sessionID: "session-1", title: "First session" },
    });
    controller.handleEvent({
      type: "session.created",
      properties: { sessionID: "session-2", title: "Second session" },
    });
    controller.handleEvent({
      type: "session.status",
      properties: { sessionID: "session-1", status: { type: "busy" } },
    });
    controller.handleEvent({
      type: "session.status",
      properties: { sessionID: "session-2", status: { type: "busy" } },
    });

    const latest = writes.at(-1);

    expect(latest?.sessions?.map((session) => session.session_title)).toEqual([
      "Second session",
      "First session",
    ]);
    expect(latest?.status).toBe("working");
    expect(latest?.session_title).toBe("Second session");
  });
});
