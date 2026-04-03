import { invoke } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { PanelShell } from "@/components/panels/PanelShell";
import { useTranslation } from "react-i18next";
import {
  BRIDGE_REQUEST_TYPE,
  BRIDGE_RESPONSE_TYPE,
  injectPluginPanelTheme,
} from "@/lib/plugin-panel-bridge";

interface ExtractedScript {
  src: string | null;
  type: string | null;
  text: string;
}

interface ParsedPanelDocument {
  markup: string;
  scripts: ExtractedScript[];
}

const THEME_VARIABLES = [
  "--space-void",
  "--space-deep",
  "--space-surface",
  "--space-overlay",
  "--text-primary",
  "--text-secondary",
  "--text-muted",
  "--glow-green",
  "--glow-sage",
  "--glow-olive",
  "--accent-orange",
  "--accent-peach",
  "--accent-teal",
  "--success",
  "--warning",
  "--danger",
  "--info",
  "--glass",
  "--glass-border",
  "--radius",
];

function currentThemeVariables(): Record<string, string> {
  const computed = getComputedStyle(document.documentElement);
  return Object.fromEntries(
    THEME_VARIABLES.map((name) => [name, computed.getPropertyValue(name).trim()]),
  );
}

function splitHtmlAndScripts(rawHtml: string): ParsedPanelDocument {
  const parsed = new DOMParser().parseFromString(rawHtml, "text/html");
  const scripts = Array.from(parsed.querySelectorAll("script")).map((script) => ({
    src: script.getAttribute("src"),
    type: script.getAttribute("type"),
    text: script.textContent ?? "",
  }));

  for (const script of Array.from(parsed.querySelectorAll("script"))) {
    script.remove();
  }

  const headMarkup = Array.from(parsed.head.children)
    .filter((node) => {
      const tagName = node.tagName.toLowerCase();
      return tagName !== "meta" && tagName !== "title";
    })
    .map((node) => node.outerHTML)
    .join("\n");

  const bodyMarkup = parsed.body.innerHTML;

  return {
    markup: `${headMarkup}\n${bodyMarkup}`.trim(),
    scripts,
  };
}

export default function PluginPanelView() {
  const { t } = useTranslation();
  const [html, setHtml] = useState<string>("");
  const [title, setTitle] = useState<string>(t("plugins.panel.defaultTitle"));
  const [error, setError] = useState<string | null>(null);
  const label = getCurrentWebviewWindow().label;
  const panelRootRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    invoke<{ label: string; title: string; width: number; height: number }[]>(
      "plugin_panels_list",
    )
      .then((panels) => {
        const panel = panels.find((entry) => entry.label === label);
        if (panel) {
          setTitle(panel.title);
        }
      })
      .catch((err) => {
        console.error("Failed to fetch plugin panel metadata:", err);
      });

    invoke<string>("plugin_panel_html", { label })
      .then((content) => {
        const finalHtml = injectPluginPanelTheme(content, currentThemeVariables());
        setHtml(finalHtml);
        setError(null);
      })
      .catch((err) => {
        setError(String(err));
      });
  }, [label]);

  useEffect(() => {
    if (!html || !panelRootRef.current) {
      return;
    }

    const { markup, scripts } = splitHtmlAndScripts(html);
    const panelRoot = panelRootRef.current;
    const blobUrls: string[] = [];
    panelRoot.innerHTML = markup;

    const scriptTarget = panelRoot;
    for (const script of scripts) {
      const runtimeScript = document.createElement("script");
      runtimeScript.async = false;
      if (script.type) {
        runtimeScript.type = script.type;
      }
      if (script.src) {
        runtimeScript.src = script.src;
      } else {
        const blobUrl = URL.createObjectURL(
          new Blob([script.text], { type: "text/javascript" }),
        );
        blobUrls.push(blobUrl);
        runtimeScript.src = blobUrl;
      }
      scriptTarget.appendChild(runtimeScript);
    }

    return () => {
      for (const blobUrl of blobUrls) {
        URL.revokeObjectURL(blobUrl);
      }
      panelRoot.innerHTML = "";
    };
  }, [html]);

  useEffect(() => {
    const handleMessage = async (event: MessageEvent) => {
      const data = event.data;
      if (!data || data.type !== BRIDGE_REQUEST_TYPE || typeof data.command !== "string") {
        return;
      }

      try {
        const result = await invoke(data.command, data.payload ?? {});
        window.postMessage(
          {
            type: BRIDGE_RESPONSE_TYPE,
            id: data.id,
            ok: true,
            result,
          },
          "*",
        );
      } catch (err) {
        window.postMessage(
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
      <PanelShell title={title}>
        <div className="flex h-full items-center justify-center text-text-secondary">
          {t("plugins.panel.failedLoad", { error })}
        </div>
      </PanelShell>
    );
  }

  return (
    <PanelShell title={title}>
      <div ref={panelRootRef} className="h-full w-full overflow-auto" />
    </PanelShell>
  );
}
