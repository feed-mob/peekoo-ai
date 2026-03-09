import { invoke } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import {
  BRIDGE_REQUEST_TYPE,
  BRIDGE_RESPONSE_TYPE,
  injectPluginPanelBridge,
} from "@/lib/plugin-panel-bridge";

export default function PluginPanelView() {
  const [html, setHtml] = useState<string>("");
  const [error, setError] = useState<string | null>(null);
  const label = getCurrentWebviewWindow().label;
  const iframeRef = useRef<HTMLIFrameElement | null>(null);

  useEffect(() => {
    invoke<string>("plugin_panel_html", { label })
      .then((content) => {
        setHtml(injectPluginPanelBridge(content));
        setError(null);
      })
      .catch((err) => {
        setError(String(err));
      });
  }, [label]);

  useEffect(() => {
    const handleMessage = async (event: MessageEvent) => {
      const data = event.data;
      if (!data || data.type !== BRIDGE_REQUEST_TYPE || typeof data.command !== "string") {
        return;
      }

      if (event.source !== iframeRef.current?.contentWindow) {
        return;
      }

      try {
        const result = await invoke(data.command, data.payload ?? {});
        iframeRef.current?.contentWindow?.postMessage(
          {
            type: BRIDGE_RESPONSE_TYPE,
            id: data.id,
            ok: true,
            result,
          },
          "*",
        );
      } catch (err) {
        iframeRef.current?.contentWindow?.postMessage(
          {
            type: BRIDGE_RESPONSE_TYPE,
            id: data.id,
            ok: false,
            error: String(err),
          },
          "*",
        );
      }
    };

    window.addEventListener("message", handleMessage);

    return () => {
      window.removeEventListener("message", handleMessage);
    };
  }, []);

  if (error) {
    return (
      <div className="flex h-screen items-center justify-center bg-glass text-text-secondary">
        Failed to load plugin panel: {error}
      </div>
    );
  }

  return (
    <iframe
      ref={iframeRef}
      title={label}
      srcDoc={html}
      className="h-screen w-full border-0 bg-transparent"
      sandbox="allow-scripts"
    />
  );
}
