import { describe, expect, test } from "bun:test";
import {
  createBridgeController,
  type BridgeWrite,
} from "./peekoo-opencode-companion";

function textMessageEvent() {
  return {
    type: "message.part.updated",
    properties: { part: { type: "text" } },
  };
}

describe("createBridgeController", () => {
  test("keeps the last known title after idle cleanup", () => {
    const writes: BridgeWrite[] = [];
    let idleTransition: (() => void) | null = null;

    const controller = createBridgeController({
      writeBridge: (state) => writes.push(state),
      scheduleIdle: (callback) => {
        idleTransition = callback;
      },
      cancelIdle: () => {},
      now: () => 100,
    });

    controller.handleEvent({
      type: "session.created",
      properties: { title: "Fix repeated badge updates" },
    });
    controller.handleEvent({
      type: "session.idle",
      properties: {},
    });

    idleTransition?.();
    controller.handleEvent(textMessageEvent());

    expect(writes.map((write) => [write.status, write.session_title])).toEqual([
      ["working", "Fix repeated badge updates"],
      ["happy", "Fix repeated badge updates"],
      ["idle", ""],
      ["working", "Fix repeated badge updates"],
    ]);
  });
});
