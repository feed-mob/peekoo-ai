import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

export function useIsDarkMode() {
  const [isDark, setIsDark] = useState(false);

  useEffect(() => {
    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    let themeMode = "system";

    const updateIsDark = (mode: string, systemDark: boolean) => {
      if (mode === "dark") {
        setIsDark(true);
      } else if (mode === "light") {
        setIsDark(false);
      } else {
        setIsDark(systemDark);
      }
    };

    // Initial load
    void invoke<Record<string, string>>("app_settings_get").then((settings) => {
      themeMode = settings.theme_mode ?? "system";
      updateIsDark(themeMode, mediaQuery.matches);
    });

    // Listen for manual changes
    const unlistenTheme = listen<{ mode: string }>("theme:changed", (event) => {
      themeMode = event.payload.mode;
      updateIsDark(themeMode, mediaQuery.matches);
    });

    // Listen for system changes
    const handler = (e: MediaQueryListEvent) => {
      updateIsDark(themeMode, e.matches);
    };
    mediaQuery.addEventListener("change", handler);

    return () => {
      mediaQuery.removeEventListener("change", handler);
      void unlistenTheme.then((fn) => fn());
    };
  }, []);

  return isDark;
}
