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
var startedAt = 0;
var bridgeTitle = "";
var lastKnownTitle = "";
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
function rememberTitle(title) {
  if (title.trim().length > 0) {
    lastKnownTitle = title;
  }
}
var happyTimeout = null;
function createBridgeController(dependencies) {
  let controllerStatus = "idle";
  let controllerBridgeTitle = "";
  let controllerLastKnownTitle = "";
  let controllerStartedAt = 0;
  const rememberControllerTitle = (title) => {
    if (title.trim().length > 0) {
      controllerLastKnownTitle = title;
    }
  };
  const emitBridge = (status, title, force = false) => {
    const changed = status !== controllerStatus || title !== controllerBridgeTitle;
    if (!changed && !force)
      return;
    controllerStatus = status;
    controllerBridgeTitle = title;
    rememberControllerTitle(title);
    if (status !== "idle" && status !== "happy") {
      if (controllerStartedAt === 0) {
        controllerStartedAt = dependencies.now();
      }
    } else {
      controllerStartedAt = 0;
    }
    dependencies.writeBridge({
      status,
      session_title: title,
      started_at: controllerStartedAt,
      updated_at: dependencies.now()
    });
  };
  const activeTitle = () => controllerLastKnownTitle;
  const handleBusy = () => {
    dependencies.cancelIdle();
    emitBridge("working", activeTitle());
  };
  const handleIdle = () => {
    emitBridge("happy", activeTitle());
    dependencies.scheduleIdle(() => {
      emitBridge("idle", "");
    });
  };
  return {
    initialize: () => {
      emitBridge("idle", "", true);
    },
    handleEvent: (event) => {
      switch (event.type) {
        case "session.status": {
          const props = event.properties;
          const statusType = props?.status?.type;
          if (statusType === "busy" || statusType === "retry") {
            handleBusy();
          } else if (statusType === "idle") {
            handleIdle();
          }
          break;
        }
        case "session.idle": {
          handleIdle();
          break;
        }
        case "session.created": {
          const title = getSessionTitle(event.properties) || "New session";
          rememberControllerTitle(title);
          dependencies.cancelIdle();
          emitBridge("working", title);
          break;
        }
        case "session.updated": {
          const title = getSessionTitle(event.properties);
          if (title) {
            rememberControllerTitle(title);
            if (controllerStatus === "working" || controllerStatus === "thinking") {
              emitBridge(controllerStatus, controllerLastKnownTitle);
            }
          }
          break;
        }
        case "session.error": {
          dependencies.cancelIdle();
          emitBridge("idle", "");
          break;
        }
        case "message.part.updated": {
          const props = event.properties;
          if (props?.part?.type === "text") {
            handleBusy();
          }
          break;
        }
      }
    }
  };
}
function persistBridgeWrite(state) {
  currentStatus = state.status;
  bridgeTitle = state.session_title;
  rememberTitle(state.session_title);
  startedAt = state.started_at;
  try {
    mkdirSync(BRIDGE_DIR, { recursive: true });
    writeFileSync(BRIDGE_FILE, JSON.stringify(state));
  } catch {}
}
var PeekooOpenCodeCompanion = async () => {
  const controller = createBridgeController({
    writeBridge: persistBridgeWrite,
    scheduleIdle: (callback) => {
      clearHappyTimeout();
      happyTimeout = setTimeout(() => {
        callback();
        happyTimeout = null;
      }, 5000);
    },
    cancelIdle: clearHappyTimeout,
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
