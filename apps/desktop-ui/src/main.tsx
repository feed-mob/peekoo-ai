import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { ResolvedView } from "@/routing/resolve-view";
import { forwardConsole } from "@/lib/log";
import "./index.css";

forwardConsole();

const label = getCurrentWebviewWindow().label;

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <ResolvedView label={label} />
  </React.StrictMode>,
);
