import { useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { PANEL_WINDOW_CONFIGS } from "@/types/window";

interface PomodoroStatus {
  mode: "work" | "break";
  state: "Idle" | "Running" | "Paused" | "Completed";
  completed_focus: number;
  enable_memo: boolean;
}

export function usePomodoroWatcher() {
  const isInitialized = useRef(false);
  const lastCompletedRef = useRef<number | null>(null);

  useEffect(() => {
    const fetchStatus = async () => {
      try {
        const res = await invoke("plugin_call_tool", {
          toolName: "pomodoro_get_status",
          argsJson: "{}"
        });
        const s: PomodoroStatus = JSON.parse(res as string);
        if (s && s.state !== undefined) {
          if (!isInitialized.current) {
            lastCompletedRef.current = s.completed_focus;
            isInitialized.current = true;
          } else {
            // Check for new completion
            if (s.state === "Completed" && s.mode === "work" && s.enable_memo) {
              if (lastCompletedRef.current !== null && lastCompletedRef.current !== s.completed_focus) {
                lastCompletedRef.current = s.completed_focus;
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
                        resizable: false,
                        shadow: true,
                      });
                    }
                  });
                }
              }
            } else if (s.state !== "Completed") {
               // keep tracker updated if running
               lastCompletedRef.current = s.completed_focus;
            }
          }
        }
      } catch (err) {
        // ignore errors if plugin not loaded
      }
    };

    void fetchStatus();
    const interval = window.setInterval(fetchStatus, 3000);
    return () => window.clearInterval(interval);
  }, []);
}
