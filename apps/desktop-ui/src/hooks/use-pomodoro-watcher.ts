import { useEffect, useRef } from "react";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import {
  getPomodoroHistory,
  getPomodoroStatus,
  type PomodoroHistoryEntry,
  type PomodoroStatus,
} from "@/features/pomodoro/tool-client";
import { PANEL_WINDOW_CONFIGS } from "@/types/window";

function hasPendingFocusMemo(entry: PomodoroHistoryEntry | undefined): boolean {
  if (!entry) return false;
  if (entry.mode !== "work") return false;
  if (!entry.memo_requested) return false;
  return !entry.memo || entry.memo.trim().length === 0;
}

function findLatestPendingFocusMemo(entries: PomodoroHistoryEntry[]): PomodoroHistoryEntry | null {
  for (const entry of entries) {
    if (hasPendingFocusMemo(entry)) {
      return entry;
    }
  }
  return null;
}

async function openPomodoroMemoWindow() {
  const config = PANEL_WINDOW_CONFIGS["panel-pomodoro-memo"];
  if (!config) return;

  const existing = await WebviewWindow.getByLabel(config.label);
  if (existing) {
    await existing.setFocus();
    return;
  }

  const webview = new WebviewWindow(config.label, {
    url: "/",
    title: config.title,
    width: config.width,
    height: config.height,
    decorations: false,
    transparent: true,
    alwaysOnTop: true,
    center: true,
    resizable: true,
    minWidth: 320,
    minHeight: 280,
    shadow: true,
    visible: false,
  });

  webview.once("tauri://created", () => {
    void webview.show().catch(() => {
      // ignore transient show failures for memo window
    });
  });
}

export function usePomodoroWatcher() {
  const isInitialized = useRef(false);
  const lastCompletedRef = useRef<number | null>(null);
  const lastPromptedCycleIdRef = useRef<string | null>(null);

  useEffect(() => {
    const maybePromptPendingMemo = async (limit: number) => {
      const history = await getPomodoroHistory(limit);
      const pendingCycle = findLatestPendingFocusMemo(history);
      if (!pendingCycle) return;
      if (pendingCycle.id === lastPromptedCycleIdRef.current) return;

      lastPromptedCycleIdRef.current = pendingCycle.id;
      await openPomodoroMemoWindow();
    };

    const fetchStatus = async () => {
      try {
        const status: PomodoroStatus = await getPomodoroStatus();
        if (!status || status.state === undefined) {
          return;
        }

        if (!isInitialized.current) {
          lastCompletedRef.current = status.completed_focus;
          isInitialized.current = true;

          if (status.enable_memo) {
            // Covers app restarts or missed transitions where completed_focus delta is not observable.
            await maybePromptPendingMemo(12);
          }
          return;
        }

        const previousCompleted = lastCompletedRef.current ?? status.completed_focus;

        if (status.completed_focus > previousCompleted) {
          lastCompletedRef.current = status.completed_focus;

          if (status.enable_memo) {
            // Scan recent history instead of only latest entry.
            await maybePromptPendingMemo(12);
          }
        } else if (status.completed_focus < previousCompleted) {
          // Daily reset or runtime reset.
          lastCompletedRef.current = status.completed_focus;
        } else if (status.enable_memo) {
          // Keep trying while memo remains unsaved, useful when WM blocks first window creation/focus.
          await maybePromptPendingMemo(12);
        }
      } catch {
        // ignore transient errors while pomodoro state is unavailable
      }
    };

    void fetchStatus();
    const interval = window.setInterval(fetchStatus, 3000);
    return () => window.clearInterval(interval);
  }, []);
}
