import ReactDOM from "react-dom/client";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { ResolvedView } from "@/routing/resolve-view";
import { shouldForwardConsole } from "@/lib/bootstrap";
import { forwardConsole } from "@/lib/log";
import { checkForAppUpdates } from "@/lib/updater";
import { useSystemTheme } from "@/hooks/use-system-theme";
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

  return <ResolvedView label={label} />;
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <App />
);
