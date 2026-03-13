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
function clearHappyTimeout() {
  if (happyTimeout) {
    clearTimeout(happyTimeout);
    happyTimeout = null;
  }
}
function getSessionTitle(properties) {
  const props = properties;
  return props?.info?.title || props?.title;
}
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
  clearHappyTimeout();
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
          const props = event.properties;
          if (props?.part?.type === "text") {
            clearHappyTimeout();
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
