import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { ResolvedView } from "@/routing/resolve-view";
import { shouldForwardConsole } from "@/lib/bootstrap";
import { forwardConsole } from "@/lib/log";
import { checkForAppUpdates } from "@/lib/updater";
import { useSystemTheme } from "@/hooks/use-system-theme";
import { initI18n, setupLanguageListener } from "@/lib/i18n";
import { useEffect } from "react";
import "./index.css";

if (shouldForwardConsole(import.meta.env.DEV)) {
  forwardConsole();
}

const label = getCurrentWebviewWindow().label;

if (label === "main") {
  void checkForAppUpdates();
}

function App() {
  useSystemTheme();
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

  return <ResolvedView label={label} />;
}

void initI18n().finally(() => {
  ReactDOM.createRoot(document.getElementById("root")!).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
});
