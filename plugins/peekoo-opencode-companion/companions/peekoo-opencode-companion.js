// peekoo-opencode-companion.ts
import { writeFileSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import { homedir } from "node:os";
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
var currentStatus = "idle";
var sessionTitle = "";
var startedAt = 0;
function writeBridge(status, title, force = false) {
  const changed = status !== currentStatus || title !== sessionTitle;
  if (!changed && !force)
    return;
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
    const state = {
      status,
      session_title: title,
      started_at: startedAt,
      updated_at: Math.floor(Date.now() / 1000)
    };
    writeFileSync(BRIDGE_FILE, JSON.stringify(state));
  } catch {}
}
var happyTimeout = null;
function scheduleIdleTransition() {
  if (happyTimeout) {
    clearTimeout(happyTimeout);
  }
  happyTimeout = setTimeout(() => {
    writeBridge("idle", "");
    happyTimeout = null;
  }, 5000);
}
var PeekooOpenCodeCompanion = async () => {
  writeBridge("idle", "", true);
  return {
    event: async ({ event }) => {
      switch (event.type) {
        case "session.status": {
          const props = event.properties;
          const status = props?.status;
          const title = props?.title || sessionTitle;
          if (status === "running") {
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
          writeBridge("happy", sessionTitle);
          scheduleIdleTransition();
          break;
        }
        case "session.created": {
          const props = event.properties;
          const title = props?.title || "New session";
          sessionTitle = title;
          writeBridge("thinking", title);
          break;
        }
        case "session.updated": {
          const props = event.properties;
          if (props?.title) {
            sessionTitle = props.title;
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
          if (currentStatus === "thinking") {
            writeBridge("working", sessionTitle);
          }
          break;
        }
      }
    }
  };
};
export {
  PeekooOpenCodeCompanion
};
