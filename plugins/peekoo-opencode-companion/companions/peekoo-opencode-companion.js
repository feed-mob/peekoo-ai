// peekoo-opencode-companion.ts
import { mkdirSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
function resolvePeekooBridgeDir() {
  if (process.platform === "win32") {
    const localAppData = process.env.LOCALAPPDATA;
    if (localAppData) {
      return join(localAppData, "Peekoo", "peekoo", "bridges");
    }
  }
  return join(homedir(), ".peekoo", "bridges");
}
var BRIDGE_DIR = resolvePeekooBridgeDir();
var BRIDGE_FILE = join(BRIDGE_DIR, "peekoo-opencode-companion.json");
var IDLE_TRANSITION_MS = 5000;
var MAX_COMPLETED_SESSIONS = 32;
function getSessionTitle(properties) {
  const props = properties;
  return props?.info?.title || props?.title;
}
function getSessionId(properties) {
  const props = properties;
  return props?.sessionID || props?.sessionId || props?.session?.id;
}
function getRequestId(properties) {
  const props = properties;
  return props?.requestID || props?.permissionID || props?.id;
}
function isActiveStatus(status) {
  return status === "working" || status === "thinking" || status === "waiting";
}
function toBridgeSession(session) {
  return {
    session_id: session.sessionId,
    status: session.status,
    session_title: session.title,
    started_at: session.startedAt,
    updated_at: session.updatedAt
  };
}
function sortByUpdatedAt(sessions) {
  return sessions.toSorted((left, right) => right.updatedAt - left.updatedAt);
}
function createBridgeController(dependencies) {
  const sessions = new Map;
  const rememberedTitles = new Map;
  const pendingCompletions = [];
  let lastBridgeSnapshot = "";
  let completionSequence = 0;
  const resolveSessionId = (event) => {
    const fromProperties = getSessionId(event.properties);
    if (fromProperties) {
      return fromProperties;
    }
    if (sessions.size === 1) {
      return sessions.keys().next().value;
    }
    return;
  };
  const resolveActiveSessionIds = (event) => {
    const sessionId = resolveSessionId(event);
    if (sessionId) {
      return [sessionId];
    }
    return [...sessions.values()].filter((session) => isActiveStatus(session.status)).map((session) => session.sessionId);
  };
  const getSession = (sessionId) => {
    return sessions.get(sessionId);
  };
  const ensureSessionForActivity = (sessionId) => {
    const existing = sessions.get(sessionId);
    if (existing) {
      return existing;
    }
    const now = dependencies.now();
    const created = {
      sessionId,
      status: "working",
      title: rememberedTitles.get(sessionId) || "OpenCode session",
      startedAt: now,
      updatedAt: now,
      pendingRequestIds: new Set
    };
    sessions.set(sessionId, created);
    return created;
  };
  const emitSnapshot = (force = false) => {
    const activeSessions = sortByUpdatedAt([...sessions.values()].filter((session) => isActiveStatus(session.status)));
    const latestCompleted = sortByUpdatedAt([...sessions.values()].filter((session) => session.status === "happy"))[0];
    const primaryActive = activeSessions[0];
    const aggregateStatus = primaryActive ? activeSessions.some((session) => session.status === "waiting") ? "waiting" : activeSessions.some((session) => session.status === "working") ? "working" : "thinking" : latestCompleted ? "happy" : "idle";
    const snapshot = {
      status: aggregateStatus,
      session_title: primaryActive?.title || latestCompleted?.title || "",
      started_at: primaryActive?.startedAt || 0,
      updated_at: dependencies.now(),
      sessions: activeSessions.map(toBridgeSession),
      completed_sessions: pendingCompletions.map((completion) => ({
        completion_id: completion.completionId,
        session_id: completion.sessionId,
        session_title: completion.title,
        updated_at: completion.updatedAt
      }))
    };
    const serialized = JSON.stringify(snapshot);
    if (!force && serialized === lastBridgeSnapshot) {
      return;
    }
    lastBridgeSnapshot = serialized;
    dependencies.writeBridge(snapshot);
  };
  const markWorking = (sessionId) => {
    dependencies.cancelIdle(sessionId);
    const session = ensureSessionForActivity(sessionId);
    const now = dependencies.now();
    session.status = "working";
    if (session.startedAt === 0) {
      session.startedAt = now;
    }
    session.updatedAt = now;
    emitSnapshot();
  };
  const markWaiting = (sessionId, requestId) => {
    dependencies.cancelIdle(sessionId);
    const session = ensureSessionForActivity(sessionId);
    if (session.pendingRequestIds.has(requestId)) {
      return;
    }
    session.pendingRequestIds.add(requestId);
    session.status = "waiting";
    session.updatedAt = dependencies.now();
    emitSnapshot();
  };
  const resolveWaiting = (sessionId, requestId) => {
    const session = getSession(sessionId);
    if (!session) {
      return;
    }
    if (!session.pendingRequestIds.delete(requestId)) {
      return;
    }
    session.status = session.pendingRequestIds.size > 0 ? "waiting" : "working";
    if (session.status === "working" && session.startedAt === 0) {
      session.startedAt = dependencies.now();
    }
    session.updatedAt = dependencies.now();
    emitSnapshot();
  };
  const markHappy = (sessionId) => {
    const session = getSession(sessionId);
    if (!session) {
      return;
    }
    if (session.status === "happy") {
      return;
    }
    session.pendingRequestIds.clear();
    session.status = "happy";
    session.startedAt = 0;
    session.updatedAt = dependencies.now();
    pendingCompletions.push({
      completionId: `${sessionId}:${session.updatedAt}:${completionSequence++}`,
      sessionId,
      title: session.title,
      updatedAt: session.updatedAt
    });
    if (pendingCompletions.length > MAX_COMPLETED_SESSIONS) {
      pendingCompletions.splice(0, pendingCompletions.length - MAX_COMPLETED_SESSIONS);
    }
    emitSnapshot();
    dependencies.scheduleIdle(sessionId, () => {
      sessions.delete(sessionId);
      emitSnapshot();
    });
  };
  return {
    initialize: () => {
      emitSnapshot(true);
    },
    handleEvent: (event) => {
      const sessionId = resolveSessionId(event);
      switch (event.type) {
        case "session.status": {
          const props = event.properties;
          const statusType = props?.status?.type;
          if ((statusType === "busy" || statusType === "retry") && sessionId) {
            markWorking(sessionId);
          } else if (statusType === "idle") {
            for (const activeSessionId of resolveActiveSessionIds(event)) {
              markHappy(activeSessionId);
            }
          }
          break;
        }
        case "session.idle": {
          for (const activeSessionId of resolveActiveSessionIds(event)) {
            markHappy(activeSessionId);
          }
          break;
        }
        case "session.created": {
          if (!sessionId) {
            break;
          }
          const title = getSessionTitle(event.properties) || "New session";
          const session = ensureSessionForActivity(sessionId);
          session.title = title;
          rememberedTitles.set(sessionId, title);
          session.status = "working";
          session.pendingRequestIds.clear();
          session.startedAt = dependencies.now();
          session.updatedAt = dependencies.now();
          dependencies.cancelIdle(sessionId);
          emitSnapshot();
          break;
        }
        case "session.updated": {
          const title = getSessionTitle(event.properties);
          if (title && sessionId) {
            const session = getSession(sessionId);
            if (!session) {
              rememberedTitles.set(sessionId, title);
              break;
            }
            session.title = title;
            rememberedTitles.set(sessionId, title);
            session.updatedAt = dependencies.now();
            emitSnapshot();
          }
          break;
        }
        case "session.error": {
          for (const activeSessionId of resolveActiveSessionIds(event)) {
            dependencies.cancelIdle(activeSessionId);
            sessions.delete(activeSessionId);
          }
          emitSnapshot();
          break;
        }
        case "permission.updated":
        case "permission.asked": {
          const requestId = getRequestId(event.properties);
          if (sessionId && requestId) {
            markWaiting(sessionId, requestId);
          }
          break;
        }
        case "permission.replied":
        case "question.replied":
        case "question.rejected": {
          const requestId = getRequestId(event.properties);
          if (sessionId && requestId) {
            resolveWaiting(sessionId, requestId);
          }
          break;
        }
        case "question.asked": {
          const requestId = getRequestId(event.properties);
          if (sessionId && requestId) {
            markWaiting(sessionId, requestId);
          }
          break;
        }
        case "message.part.updated": {
          const props = event.properties;
          if (props?.part?.type === "text" && sessionId) {
            markWorking(sessionId);
          }
          break;
        }
      }
    }
  };
}
var happyTimeouts = new Map;
function persistBridgeWrite(state) {
  try {
    mkdirSync(BRIDGE_DIR, { recursive: true });
    writeFileSync(BRIDGE_FILE, JSON.stringify(state));
  } catch {}
}
function cancelIdleTransition(sessionId) {
  const timeout = happyTimeouts.get(sessionId);
  if (timeout) {
    clearTimeout(timeout);
    happyTimeouts.delete(sessionId);
  }
}
var PeekooOpenCodeCompanion = async () => {
  const controller = createBridgeController({
    writeBridge: persistBridgeWrite,
    scheduleIdle: (sessionId, callback) => {
      cancelIdleTransition(sessionId);
      const timeout = setTimeout(() => {
        callback();
        happyTimeouts.delete(sessionId);
      }, IDLE_TRANSITION_MS);
      happyTimeouts.set(sessionId, timeout);
    },
    cancelIdle: cancelIdleTransition,
    now: () => Math.floor(Date.now() / 1000)
  });
  controller.initialize();
  return {
    event: async ({ event }) => {
      controller.handleEvent(event);
    }
  };
};
export {
  createBridgeController,
  PeekooOpenCodeCompanion
};
