import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { Effects } from "@tauri-apps/api/window";

const TRANSPARENT_BACKGROUND = "#00000000";
const PANEL_LABEL_PREFIX = "panel-";
const MACOS_PANEL_WINDOW_EFFECTS: Effects = {
  effects: ["hudWindow" as Effects["effects"][number]],
  state: "active" as NonNullable<Effects["state"]>,
  radius: 24,
};

export function isMacOsPlatform(userAgent: string = navigator.userAgent): boolean {
  return /Macintosh|Mac OS X/i.test(userAgent);
}

export function panelWindowEffects() {
  return isMacOsPlatform() ? MACOS_PANEL_WINDOW_EFFECTS : undefined;
}

export function panelWindowBackgroundColor(): string {
  return TRANSPARENT_BACKGROUND;
}

function applyTransparentRootStyles() {
  document.documentElement.style.backgroundColor = "transparent";
  document.documentElement.style.background = "transparent";
  document.body.style.backgroundColor = "transparent";
  document.body.style.background = "transparent";

  const root = document.getElementById("root");
  if (root) {
    root.style.backgroundColor = "transparent";
    root.style.background = "transparent";
  }
}

async function syncWindowTransparency(webviewWindow: WebviewWindow) {
  applyTransparentRootStyles();
  await webviewWindow.setBackgroundColor(TRANSPARENT_BACKGROUND);
}

export function installMacOsPanelTransparencyFix(
  webviewWindow: WebviewWindow = getCurrentWebviewWindow(),
): () => void {
  if (!isMacOsPlatform() || !webviewWindow.label.startsWith(PANEL_LABEL_PREFIX)) {
    return () => {};
  }

  let disposed = false;
  let rafId: number | null = null;
  let timeoutId: number | null = null;
  const cleanupCallbacks: Array<() => void> = [];

  const flush = () => {
    if (disposed) {
      return;
    }

    void syncWindowTransparency(webviewWindow).catch((error) => {
      console.warn(`Failed to keep ${webviewWindow.label} transparent on macOS`, error);
    });
  };

  const schedule = () => {
    if (disposed || rafId !== null) {
      return;
    }

    rafId = window.requestAnimationFrame(() => {
      rafId = null;
      flush();

      if (timeoutId !== null) {
        window.clearTimeout(timeoutId);
      }

      timeoutId = window.setTimeout(() => {
        timeoutId = null;
        flush();
      }, 48);
    });
  };

  schedule();
  cleanupCallbacks.push(() => {
    if (rafId !== null) {
      window.cancelAnimationFrame(rafId);
      rafId = null;
    }
    if (timeoutId !== null) {
      window.clearTimeout(timeoutId);
      timeoutId = null;
    }
  });

  const mutationObserver = new MutationObserver(schedule);
  mutationObserver.observe(document.documentElement, {
    attributes: true,
    childList: true,
    subtree: true,
  });
  cleanupCallbacks.push(() => mutationObserver.disconnect());

  const resizeObserver = new ResizeObserver(schedule);
  resizeObserver.observe(document.documentElement);
  cleanupCallbacks.push(() => resizeObserver.disconnect());

  document.addEventListener("visibilitychange", schedule);
  cleanupCallbacks.push(() => document.removeEventListener("visibilitychange", schedule));

  for (const eventName of ["tauri://move", "tauri://resize", "tauri://focus", "tauri://blur"]) {
    void webviewWindow.listen(eventName, schedule).then((unlisten) => {
      if (disposed) {
        unlisten();
        return;
      }
      cleanupCallbacks.push(unlisten);
    });
  }

  return () => {
    disposed = true;
    for (const callback of cleanupCallbacks.splice(0)) {
      callback();
    }
  };
}
