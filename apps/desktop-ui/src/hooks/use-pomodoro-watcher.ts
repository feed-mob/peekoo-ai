import { useEffect, useRef } from "react";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { getPomodoroStatus, type PomodoroStatus } from "@/features/pomodoro/tool-client";
import { PANEL_WINDOW_CONFIGS } from "@/types/window";

export function usePomodoroWatcher() {
  const isInitialized = useRef(false);
  const lastCompletedRef = useRef<number | null>(null);

  useEffect(() => {
    const fetchStatus = async () => {
      try {
        const status: PomodoroStatus = await getPomodoroStatus();
        if (!status || status.state === undefined) {
          return;
        }

        if (!isInitialized.current) {
          lastCompletedRef.current = status.completed_focus;
          isInitialized.current = true;
          return;
        }

        if (status.state === "Completed" && status.mode === "work" && status.enable_memo) {
          if (lastCompletedRef.current !== null && lastCompletedRef.current !== status.completed_focus) {
            lastCompletedRef.current = status.completed_focus;
            const config = PANEL_WINDOW_CONFIGS["panel-pomodoro-memo"];
            if (config) {
              void WebviewWindow.getByLabel(config.label).then(async (existing) => {
                if (existing) {
                  await existing.setFocus();
                } else {
                  new WebviewWindow(config.label, {
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
                  });
                }
              });
            }
          }
        } else if (status.state !== "Completed") {
          lastCompletedRef.current = status.completed_focus;
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
