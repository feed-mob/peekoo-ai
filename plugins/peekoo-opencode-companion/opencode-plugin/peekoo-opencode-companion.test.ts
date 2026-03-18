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

  test("ignores standalone session updates for unknown sessions", () => {
    const writes: BridgeWrite[] = [];

    const controller = createBridgeController({
      writeBridge: (state) => writes.push(state),
      scheduleIdle: () => {},
      cancelIdle: () => {},
      now: () => 300,
    });

    controller.handleEvent({
      type: "session.updated",
      properties: { sessionID: "session-ghost", title: "Renamed only" },
    });

    expect(writes).toEqual([]);
  });

  test("keeps a completion marker when another session is still active", () => {
    const writes: BridgeWrite[] = [];

    const controller = createBridgeController({
      writeBridge: (state) => writes.push(state),
      scheduleIdle: () => {},
      cancelIdle: () => {},
      now: (() => {
        let current = 400;
        return () => current++;
      })(),
    });

    controller.handleEvent({
      type: "session.created",
      properties: { sessionID: "session-a", title: "Finish A" },
    });
    controller.handleEvent({
      type: "session.created",
      properties: { sessionID: "session-b", title: "Keep B running" },
    });
    controller.handleEvent({
      type: "session.idle",
      properties: { sessionID: "session-a" },
    });

    const latest = writes.at(-1);

    expect(latest?.status).toBe("working");
    expect(latest?.sessions.map((session) => session.session_id)).toEqual(["session-b"]);
    expect(latest?.completed_sessions?.map((session) => session.session_id)).toEqual([
      "session-a",
    ]);
    expect(latest?.completed_sessions?.[0]?.session_title).toBe("Finish A");
  });

  test("clears all active sessions when a global idle event arrives without a session id", () => {
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
      now: (() => {
        let current = 500;
        return () => current++;
      })(),
    });

    controller.handleEvent({
      type: "session.created",
      properties: { sessionID: "session-a", title: "Fixing peek badge" },
    });
    controller.handleEvent({
      type: "session.created",
      properties: { sessionID: "session-b", title: "OpenCode task" },
    });
    controller.handleEvent({
      type: "session.idle",
      properties: {},
    });

    expect(writes.at(-1)?.sessions).toEqual([]);
    expect(idleTransitions.size).toBe(2);
    expect(writes.at(-1)?.status).toBe("happy");
  });

  test("preserves all completions when multiple sessions finish between polls", () => {
    const writes: BridgeWrite[] = [];

    const controller = createBridgeController({
      writeBridge: (state) => writes.push(state),
      scheduleIdle: () => {},
      cancelIdle: () => {},
      now: (() => {
        let current = 600;
        return () => current++;
      })(),
    });

    controller.handleEvent({
      type: "session.created",
      properties: { sessionID: "session-a", title: "First task" },
    });
    controller.handleEvent({
      type: "session.created",
      properties: { sessionID: "session-b", title: "Second task" },
    });
    controller.handleEvent({
      type: "session.idle",
      properties: { sessionID: "session-a" },
    });
    controller.handleEvent({
      type: "session.idle",
      properties: { sessionID: "session-b" },
    });

    const latest = writes.at(-1);

    expect(latest?.status).toBe("happy");
    expect(latest?.completed_sessions?.length).toBe(2);
    expect(latest?.completed_sessions?.map((c) => c.session_id)).toEqual([
      "session-a",
      "session-b",
    ]);
  });

  test("single session completion emits a completed_sessions entry", () => {
    const writes: BridgeWrite[] = [];

    const controller = createBridgeController({
      writeBridge: (state) => writes.push(state),
      scheduleIdle: () => {},
      cancelIdle: () => {},
      now: (() => {
        let current = 700;
        return () => current++;
      })(),
    });

    controller.handleEvent({
      type: "session.created",
      properties: { sessionID: "session-x", title: "My task" },
    });
    controller.handleEvent({
      type: "session.idle",
      properties: { sessionID: "session-x" },
    });

    const latest = writes.at(-1);

    expect(latest?.status).toBe("happy");
    expect(latest?.completed_sessions?.length).toBe(1);
    expect(latest?.completed_sessions?.[0]?.session_id).toBe("session-x");
    expect(latest?.completed_sessions?.[0]?.session_title).toBe("My task");
  });

  test("does not enqueue duplicate completions for repeated terminal events", () => {
    const writes: BridgeWrite[] = [];

    const controller = createBridgeController({
      writeBridge: (state) => writes.push(state),
      scheduleIdle: () => {},
      cancelIdle: () => {},
      now: (() => {
        let current = 800;
        return () => current++;
      })(),
    });

    controller.handleEvent({
      type: "session.created",
      properties: { sessionID: "session-z", title: "Quick check-in" },
    });
    controller.handleEvent({
      type: "session.status",
      properties: { sessionID: "session-z", status: { type: "idle" } },
    });
    controller.handleEvent({
      type: "session.idle",
      properties: { sessionID: "session-z" },
    });

    const latest = writes.at(-1);

    expect(latest?.completed_sessions?.length).toBe(1);
    expect(latest?.completed_sessions?.[0]?.session_id).toBe("session-z");
  });

  test("marks a session as waiting when permission is asked", () => {
    const writes: BridgeWrite[] = [];

    const controller = createBridgeController({
      writeBridge: (state) => writes.push(state),
      scheduleIdle: () => {},
      cancelIdle: () => {},
      now: (() => {
        let current = 900;
        return () => current++;
      })(),
    });

    controller.handleEvent({
      type: "session.created",
      properties: { sessionID: "session-wait", title: "Needs approval" },
    });
    controller.handleEvent({
      type: "permission.asked",
      properties: { sessionID: "session-wait", requestID: "perm-1" },
    });

    const latest = writes.at(-1);
    expect(latest?.status).toBe("waiting");
    expect(latest?.sessions[0]?.status).toBe("waiting");
  });

  test("returns to working after the last pending request is answered", () => {
    const writes: BridgeWrite[] = [];

    const controller = createBridgeController({
      writeBridge: (state) => writes.push(state),
      scheduleIdle: () => {},
      cancelIdle: () => {},
      now: (() => {
        let current = 1000;
        return () => current++;
      })(),
    });

    controller.handleEvent({
      type: "session.created",
      properties: { sessionID: "session-wait", title: "Needs approval" },
    });
    controller.handleEvent({
      type: "permission.asked",
      properties: { sessionID: "session-wait", requestID: "perm-1" },
    });
    controller.handleEvent({
      type: "permission.replied",
      properties: { sessionID: "session-wait", requestID: "perm-1", reply: "once" },
    });

    const latest = writes.at(-1);
    expect(latest?.status).toBe("working");
    expect(latest?.sessions[0]?.status).toBe("working");
  });

  test("keeps waiting until all pending questions and permissions are resolved", () => {
    const writes: BridgeWrite[] = [];

    const controller = createBridgeController({
      writeBridge: (state) => writes.push(state),
      scheduleIdle: () => {},
      cancelIdle: () => {},
      now: (() => {
        let current = 1100;
        return () => current++;
      })(),
    });

    controller.handleEvent({
      type: "session.created",
      properties: { sessionID: "session-wait", title: "Needs input" },
    });
    controller.handleEvent({
      type: "permission.asked",
      properties: { sessionID: "session-wait", requestID: "perm-1" },
    });
    controller.handleEvent({
      type: "question.asked",
      properties: { sessionID: "session-wait", id: "question-1", questions: [] },
    });
    controller.handleEvent({
      type: "permission.replied",
      properties: { sessionID: "session-wait", requestID: "perm-1", reply: "once" },
    });

    expect(writes.at(-1)?.status).toBe("waiting");

    controller.handleEvent({
      type: "question.replied",
      properties: { sessionID: "session-wait", requestID: "question-1", answers: [] },
    });

    expect(writes.at(-1)?.status).toBe("working");
  });
});
