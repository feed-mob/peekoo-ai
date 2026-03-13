import type { Plugin } from "@opencode-ai/plugin";
import { writeFileSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import { homedir } from "node:os";

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

type Status = "working" | "thinking" | "happy" | "idle";

interface BridgeState {
  status: Status;
  session_title: string;
  started_at: number;
  updated_at: number;
}

interface SessionInfo {
  title?: string;
}

interface SessionStatusInfo {
  type?: "busy" | "idle" | "retry";
}

let currentStatus: Status = "idle";
let sessionTitle = "";
let startedAt = 0;

function clearHappyTimeout(): void {
  if (happyTimeout) {
    clearTimeout(happyTimeout);
    happyTimeout = null;
  }
}

function getSessionTitle(properties: unknown): string | undefined {
  const props = properties as { info?: SessionInfo; title?: string } | undefined;
  return props?.info?.title || props?.title;
}

function writeBridge(status: Status, title: string, force = false): void {
  const changed = status !== currentStatus || title !== sessionTitle;
  if (!changed && !force) return;

  currentStatus = status;
  sessionTitle = title;

  if (status !== "idle" && status !== "happy") {
    if (startedAt === 0) {
      startedAt = Math.floor(Date.now() / 1000);
    }
  } else {
    startedAt = 0;
  }

  try {
    mkdirSync(BRIDGE_DIR, { recursive: true });

    const state: BridgeState = {
      status,
      session_title: title,
      started_at: startedAt,
      updated_at: Math.floor(Date.now() / 1000),
    };

    writeFileSync(BRIDGE_FILE, JSON.stringify(state));
  } catch {
    // Silently ignore write errors — Peekoo may not be installed
  }
}

let happyTimeout: ReturnType<typeof setTimeout> | null = null;

function scheduleIdleTransition(): void {
  clearHappyTimeout();
  happyTimeout = setTimeout(() => {
    writeBridge("idle", "");
    happyTimeout = null;
  }, 5000);
}

export const PeekooOpenCodeCompanion: Plugin = async () => {
  // Force-write idle on startup to clear any stale bridge state from a
  // previous run or crash. Without force=true this would be a no-op since
  // the in-memory state already starts as "idle".
  writeBridge("idle", "", true);

  return {
    event: async ({ event }) => {
      switch (event.type) {
        case "session.status": {
          const props = event.properties as
            | { status?: SessionStatusInfo; sessionID?: string }
            | undefined;
          const statusType = props?.status?.type;

          if (statusType === "busy") {
            clearHappyTimeout();
            writeBridge("working", sessionTitle);
          } else if (statusType === "retry") {
            clearHappyTimeout();
            writeBridge("working", sessionTitle);
          } else if (statusType === "idle") {
            writeBridge("happy", sessionTitle);
            scheduleIdleTransition();
          }
          break;
        }

        case "session.idle": {
          writeBridge("happy", sessionTitle);
          scheduleIdleTransition();
          break;
        }

        case "session.created": {
          const title = getSessionTitle(event.properties) || "New session";
          sessionTitle = title;
          clearHappyTimeout();
          writeBridge("working", title);
          break;
        }

        case "session.updated": {
          const title = getSessionTitle(event.properties);
          if (title) {
            sessionTitle = title;
            if (currentStatus === "working" || currentStatus === "thinking") {
              writeBridge(currentStatus, sessionTitle);
            }
          }
          break;
        }

        case "session.error": {
          clearHappyTimeout();
          writeBridge("idle", "");
          break;
        }

        case "message.part.updated": {
          const props = event.properties as
            | { part?: { type?: string }; delta?: string }
            | undefined;

          if (props?.part?.type === "text") {
            clearHappyTimeout();
            writeBridge("working", sessionTitle);
          }
          break;
        }
      }
    },
  };
};
