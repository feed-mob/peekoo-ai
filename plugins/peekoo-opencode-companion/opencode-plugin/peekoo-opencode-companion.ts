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

let currentStatus: Status = "idle";
let sessionTitle = "";
let startedAt = 0;

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
  if (happyTimeout) {
    clearTimeout(happyTimeout);
  }
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
            | { status?: string; title?: string }
            | undefined;
          const status = props?.status;
          const title = props?.title || sessionTitle;

          if (status === "running") {
            // Cancel any pending idle transition — we're active again
            if (happyTimeout) {
              clearTimeout(happyTimeout);
              happyTimeout = null;
            }
            writeBridge("working", title);
          } else if (status === "pending") {
            if (happyTimeout) {
              clearTimeout(happyTimeout);
              happyTimeout = null;
            }
            writeBridge("thinking", title);
          }
          break;
        }

        case "session.idle": {
          // Agent finished — show happy, then transition to idle
          writeBridge("happy", sessionTitle);
          scheduleIdleTransition();
          break;
        }

        case "session.created": {
          const props = event.properties as { title?: string } | undefined;
          const title = props?.title || "New session";
          sessionTitle = title;
          writeBridge("thinking", title);
          break;
        }

        case "session.updated": {
          const props = event.properties as { title?: string } | undefined;
          if (props?.title) {
            sessionTitle = props.title;
            // If we're in an active state, update the bridge with new title
            if (currentStatus === "working" || currentStatus === "thinking") {
              writeBridge(currentStatus, sessionTitle);
            }
          }
          break;
        }

        case "session.error": {
          writeBridge("idle", "");
          break;
        }

        case "message.part.updated": {
          // LLM is actively streaming — promote thinking to working
          if (currentStatus === "thinking") {
            writeBridge("working", sessionTitle);
          }
          break;
        }
      }
    },
  };
};
