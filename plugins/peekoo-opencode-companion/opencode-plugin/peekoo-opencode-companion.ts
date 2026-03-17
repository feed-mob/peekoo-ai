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

export interface BridgeWrite {
  status: Status;
  session_title: string;
  started_at: number;
  updated_at: number;
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

let currentStatus: Status = "idle";
let startedAt = 0;
let bridgeTitle = "";
let lastKnownTitle = "";

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

function rememberTitle(title: string): void {
  if (title.trim().length > 0) {
    lastKnownTitle = title;
  }
}

let happyTimeout: ReturnType<typeof setTimeout> | null = null;

interface BridgeControllerDependencies {
  writeBridge: (state: BridgeWrite) => void;
  scheduleIdle: (callback: () => void) => void;
  cancelIdle: () => void;
  now: () => number;
}

interface BridgeController {
  initialize: () => void;
  handleEvent: (event: BridgeEvent) => void;
}

export function createBridgeController(
  dependencies: BridgeControllerDependencies,
): BridgeController {
  let controllerStatus: Status = "idle";
  let controllerBridgeTitle = "";
  let controllerLastKnownTitle = "";
  let controllerStartedAt = 0;

  const rememberControllerTitle = (title: string): void => {
    if (title.trim().length > 0) {
      controllerLastKnownTitle = title;
    }
  };

  const emitBridge = (status: Status, title: string, force = false): void => {
    const changed =
      status !== controllerStatus || title !== controllerBridgeTitle;
    if (!changed && !force) return;

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
      updated_at: dependencies.now(),
    });
  };

  const activeTitle = (): string => controllerLastKnownTitle;

  const handleBusy = (): void => {
    dependencies.cancelIdle();
    emitBridge("working", activeTitle());
  };

  const handleIdle = (): void => {
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
          const props = event.properties as
            | { status?: SessionStatusInfo; sessionID?: string }
            | undefined;
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
          const props = event.properties as
            | { part?: { type?: string }; delta?: string }
            | undefined;

          if (props?.part?.type === "text") {
            handleBusy();
          }
          break;
        }
      }
    },
  };
}

function persistBridgeWrite(state: BridgeWrite): void {
  currentStatus = state.status;
  bridgeTitle = state.session_title;
  rememberTitle(state.session_title);
  startedAt = state.started_at;

  try {
    mkdirSync(BRIDGE_DIR, { recursive: true });
    writeFileSync(BRIDGE_FILE, JSON.stringify(state));
  } catch {
    // Silently ignore write errors — Peekoo may not be installed
  }
}

export const PeekooOpenCodeCompanion: Plugin = async () => {
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
    now: () => Math.floor(Date.now() / 1000),
  });

  controller.initialize();

  return {
    event: async ({ event }) => {
      controller.handleEvent(event);
    },
  };
};
