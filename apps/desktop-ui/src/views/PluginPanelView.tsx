import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

export default function PluginPanelView() {
  const [html, setHtml] = useState<string>("");
  const [error, setError] = useState<string | null>(null);
  const label = getCurrentWebviewWindow().label;

  useEffect(() => {
    invoke<string>("plugin_panel_html", { label })
      .then((content) => {
        setHtml(content);
        setError(null);
      })
      .catch((err) => {
        setError(String(err));
      });
  }, [label]);

  if (error) {
    return (
      <div className="flex h-screen items-center justify-center bg-glass text-text-secondary">
        Failed to load plugin panel: {error}
      </div>
    );
  }

  return (
    <iframe
      title={label}
      srcDoc={html}
      className="h-screen w-full border-0 bg-transparent"
      sandbox="allow-scripts"
    />
  );
}
