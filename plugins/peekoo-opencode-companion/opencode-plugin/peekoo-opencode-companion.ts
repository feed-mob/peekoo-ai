import type { Plugin } from "@opencode-ai/plugin";
import { mkdirSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

function resolvePeekooBridgeDir(): string {
  if (process.platform === "win32") {
    const localAppData = process.env.LOCALAPPDATA;
    if (localAppData) {
      return join(localAppData, "Peekoo", "peekoo", "bridges");
    }
  }

  return join(homedir(), ".peekoo", "bridges");
}

const BRIDGE_DIR = resolvePeekooBridgeDir();
const BRIDGE_FILE = join(BRIDGE_DIR, "peekoo-opencode-companion.json");
const IDLE_TRANSITION_MS = 5000;
const MAX_COMPLETED_SESSIONS = 32;

type Status = "working" | "thinking" | "waiting" | "happy" | "idle";

export interface BridgeSessionWrite {
  session_id: string;
  status: Status;
  session_title: string;
  started_at: number;
  updated_at: number;
}

export interface CompletedSessionWrite {
  completion_id: string;
  session_id: string;
  session_title: string;
  updated_at: number;
}

export interface BridgeWrite {
  status: Status;
  session_title: string;
  started_at: number;
  updated_at: number;
  sessions: BridgeSessionWrite[];
  completed_sessions: CompletedSessionWrite[];
}

interface BridgeEvent {
  type: string;
  properties?: unknown;
}

interface SessionInfo {
  title?: string;
}

interface SessionStatusInfo {
  type?: "busy" | "idle" | "retry";
}

interface SessionRecord {
  sessionId: string;
  status: Status;
  title: string;
  startedAt: number;
  updatedAt: number;
  pendingRequestIds: Set<string>;
}

interface CompletedSessionRecord {
  completionId: string;
  sessionId: string;
  title: string;
  updatedAt: number;
}

interface BridgeControllerDependencies {
  writeBridge: (state: BridgeWrite) => void;
  scheduleIdle: (sessionId: string, callback: () => void) => void;
  cancelIdle: (sessionId: string) => void;
  now: () => number;
}

interface BridgeController {
  initialize: () => void;
  handleEvent: (event: BridgeEvent) => void;
}

function getSessionTitle(properties: unknown): string | undefined {
  const props = properties as { info?: SessionInfo; title?: string } | undefined;
  return props?.info?.title || props?.title;
}

// OpenCode events provide the session identifier under varying keys depending
// on the event type.  We intentionally omit a bare `id` fallback to avoid
// accidentally matching unrelated identifiers (e.g. message or permission IDs).
function getSessionId(properties: unknown): string | undefined {
  const props = properties as
    | {
        sessionID?: string;
        sessionId?: string;
        session?: { id?: string };
      }
    | undefined;

  return props?.sessionID || props?.sessionId || props?.session?.id;
}

// Permission and question events use `requestID` or `permissionID` as the
// request identifier.  Both are tracked uniformly so a single pending-request
// set can cover permissions and questions alike.  `question.asked` events may
// carry the identifier under a bare `id` key, so it is kept as a last-resort
// fallback.
function getRequestId(properties: unknown): string | undefined {
  const props = properties as
    | {
        requestID?: string;
        permissionID?: string;
        id?: string;
      }
    | undefined;

  return props?.requestID || props?.permissionID || props?.id;
}

function isActiveStatus(status: Status): boolean {
  return status === "working" || status === "thinking" || status === "waiting";
}

function toBridgeSession(session: SessionRecord): BridgeSessionWrite {
  return {
    session_id: session.sessionId,
    status: session.status,
    session_title: session.title,
    started_at: session.startedAt,
    updated_at: session.updatedAt,
  };
}

function sortByUpdatedAt(sessions: SessionRecord[]): SessionRecord[] {
  return sessions.toSorted((left, right) => right.updatedAt - left.updatedAt);
}

export function createBridgeController(
  dependencies: BridgeControllerDependencies,
): BridgeController {
  const sessions = new Map<string, SessionRecord>();
  const rememberedTitles = new Map<string, string>();
  const pendingCompletions: CompletedSessionRecord[] = [];
  let lastBridgeSnapshot = "";
  let completionSequence = 0;

  const resolveSessionId = (event: BridgeEvent): string | undefined => {
    const fromProperties = getSessionId(event.properties);
    if (fromProperties) {
      return fromProperties;
    }

    if (sessions.size === 1) {
      return sessions.keys().next().value;
    }

    return undefined;
  };

  const resolveActiveSessionIds = (event: BridgeEvent): string[] => {
    const sessionId = resolveSessionId(event);
    if (sessionId) {
      return [sessionId];
    }

    return [...sessions.values()]
      .filter((session) => isActiveStatus(session.status))
      .map((session) => session.sessionId);
  };

  const getSession = (sessionId: string): SessionRecord | undefined => {
    return sessions.get(sessionId);
  };

  const ensureSessionForActivity = (sessionId: string): SessionRecord => {
    const existing = sessions.get(sessionId);
    if (existing) {
      return existing;
    }

    const now = dependencies.now();
    const created: SessionRecord = {
      sessionId,
      status: "working",
      title: rememberedTitles.get(sessionId) || "OpenCode session",
      startedAt: now,
      updatedAt: now,
      pendingRequestIds: new Set(),
    };
    sessions.set(sessionId, created);
    return created;
  };

  const emitSnapshot = (force = false): void => {
    const activeSessions = sortByUpdatedAt(
      [...sessions.values()].filter((session) => isActiveStatus(session.status)),
    );
    const latestCompleted = sortByUpdatedAt(
      [...sessions.values()].filter((session) => session.status === "happy"),
    )[0];

    const primaryActive = activeSessions[0];
    const aggregateStatus: Status = primaryActive
      ? activeSessions.some((session) => session.status === "waiting")
        ? "waiting"
        : activeSessions.some((session) => session.status === "working")
        ? "working"
        : "thinking"
      : latestCompleted
        ? "happy"
        : "idle";

    const snapshot: BridgeWrite = {
      status: aggregateStatus,
      session_title: primaryActive?.title || latestCompleted?.title || "",
      started_at: primaryActive?.startedAt || 0,
      updated_at: dependencies.now(),
      sessions: activeSessions.map(toBridgeSession),
      completed_sessions: pendingCompletions.map((completion) => ({
        completion_id: completion.completionId,
        session_id: completion.sessionId,
        session_title: completion.title,
        updated_at: completion.updatedAt,
      })),
    };

    const serialized = JSON.stringify(snapshot);
    if (!force && serialized === lastBridgeSnapshot) {
      return;
    }

    lastBridgeSnapshot = serialized;
    dependencies.writeBridge(snapshot);
  };

  const markWorking = (sessionId: string): void => {
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

  const markWaiting = (sessionId: string, requestId: string): void => {
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

  const resolveWaiting = (sessionId: string, requestId: string): void => {
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

  const markHappy = (sessionId: string): void => {
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
      updatedAt: session.updatedAt,
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
          const props = event.properties as { status?: SessionStatusInfo } | undefined;
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
          const props = event.properties as { part?: { type?: string } } | undefined;

          if (props?.part?.type === "text" && sessionId) {
            markWorking(sessionId);
          }
          break;
        }
      }
    },
  };
}

const happyTimeouts = new Map<string, ReturnType<typeof setTimeout>>();

function persistBridgeWrite(state: BridgeWrite): void {
  try {
    mkdirSync(BRIDGE_DIR, { recursive: true });
    writeFileSync(BRIDGE_FILE, JSON.stringify(state));
  } catch {
    // Silently ignore write errors — Peekoo may not be installed
  }
}

function cancelIdleTransition(sessionId: string): void {
  const timeout = happyTimeouts.get(sessionId);
  if (timeout) {
    clearTimeout(timeout);
    happyTimeouts.delete(sessionId);
  }
}

export const PeekooOpenCodeCompanion: Plugin = async () => {
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
    now: () => Math.floor(Date.now() / 1000),
  });

  controller.initialize();

  return {
    event: async ({ event }) => {
      controller.handleEvent(event);
    },
  };
};
