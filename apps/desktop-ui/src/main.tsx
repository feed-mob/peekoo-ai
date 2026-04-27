import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { ResolvedView } from "@/routing/resolve-view";
import { shouldForwardConsole } from "@/lib/bootstrap";
import { forwardConsole } from "@/lib/log";
import { checkForAppUpdates } from "@/lib/updater";
import { useSystemTheme } from "@/hooks/use-system-theme";
import { initI18n, setupLanguageListener } from "@/lib/i18n";
import { openPanelWindow } from "@/hooks/use-panel-windows";
import { useEffect } from "react";
import { installMacOsPanelTransparencyFix } from "@/lib/window-transparency";
import "./index.css";

if (shouldForwardConsole(import.meta.env.DEV)) {
  forwardConsole();
}

const currentWindow = getCurrentWebviewWindow();
const label = currentWindow.label;
const FORCE_UPDATER_IN_DEV = import.meta.env.DEV && import.meta.env.VITE_FORCE_UPDATER_DIALOG === "true";

function App() {
  useSystemTheme();

  useEffect(() => installMacOsPanelTransparencyFix(currentWindow), []);

  useEffect(() => {
    let disposed = false;
    let teardown: (() => void) | null = null;
    void setupLanguageListener().then((unlisten) => {
      if (disposed) {
        unlisten();
        return;
      }
      teardown = unlisten;
    });

    return () => {
      disposed = true;
      teardown?.();
    };
  }, []);

  useEffect(() => {
    if (label !== "main") {
      return;
    }

    void currentWindow.show().catch((error) => {
      console.error("Failed to show main window", error);
    });

    let active = true;

    void checkForAppUpdates({ forceInDev: FORCE_UPDATER_IN_DEV }).then((nextUpdateInfo) => {
      if (!active || !nextUpdateInfo) {
        return;
      }

      void openPanelWindow("panel-updater");
    });

    return () => {
      active = false;
    };
  }, []);

  return <ResolvedView label={label} />;
}

void initI18n().finally(() => {
  ReactDOM.createRoot(document.getElementById("root")!).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
});
