import { useState, useCallback } from "react";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { PanelLabel } from "@/types/window";
import { PANEL_WINDOW_CONFIGS } from "@/types/window";
import { emitPetReaction } from "@/lib/pet-events";
import { usePlugins } from "@/hooks/use-plugins";
import type { PluginPanel } from "@/types/plugin";
import type { PanelWindowConfig } from "@/types/window";

interface PanelWindowState {
  isOpen: boolean;
}

export type PanelWindowStates = Record<string, PanelWindowState>;

const INITIAL_STATE: PanelWindowStates = {
  "panel-chat": { isOpen: false },
  "panel-tasks": { isOpen: false },
  "panel-pomodoro": { isOpen: false },
  "panel-plugins": { isOpen: false },
};

const PANEL_OFFSET_X = 20;

function resolvePanelConfig(
  label: string,
  pluginPanels: PluginPanel[],
): PanelWindowConfig | PluginPanel | undefined {
  return PANEL_WINDOW_CONFIGS[label] ?? pluginPanels.find((panel) => panel.label === label);
}

export async function openPanelWindow(
  label: PanelLabel,
  pluginPanels: PluginPanel[] = [],
): Promise<WebviewWindow> {
  const config = resolvePanelConfig(label, pluginPanels);
  if (!config) {
    throw new Error(`Unknown panel config: ${label}`);
  }

  const existing = await WebviewWindow.getByLabel(label);
  if (existing) {
    await existing.setFocus();
    return existing;
  }

  const spriteWindow = getCurrentWindow();
  const spritePos = await spriteWindow.outerPosition();
  const spriteSize = await spriteWindow.outerSize();

  const panelX = spritePos.x + spriteSize.width + PANEL_OFFSET_X;
  const panelY = spritePos.y;

  return new WebviewWindow(label, {
    url: "/",
    title: config.title,
    width: config.width,
    height: config.height,
    x: panelX,
    y: panelY,
    decorations: false,
    transparent: true,
    alwaysOnTop: true,
    skipTaskbar: true,
    resizable: false,
  });
}

export async function closePanelWindow(label: PanelLabel): Promise<void> {
  const existing = await WebviewWindow.getByLabel(label);
  if (existing) {
    await existing.close();
  }
}

export function usePanelWindows() {
  const { panels: pluginPanels } = usePlugins();
  const [panels, setPanels] = useState<PanelWindowStates>(INITIAL_STATE);

  const openPanel = useCallback(async (label: PanelLabel) => {
    const existing = await WebviewWindow.getByLabel(label);
    if (existing) {
      await existing.setFocus();
      setPanels((prev) => ({ ...prev, [label]: { isOpen: true } }));
      void emitPetReaction("panel-opened");
      return;
    }

    const webview = await openPanelWindow(label, pluginPanels);

    webview.once("tauri://created", () => {
      setPanels((prev) => ({ ...prev, [label]: { isOpen: true } }));
      void emitPetReaction("panel-opened");
    });

    webview.once("tauri://error", (e) => {
      console.error(`Failed to create panel ${label}:`, e);
    });

    webview.once("tauri://destroyed", () => {
      setPanels((prev) => ({ ...prev, [label]: { isOpen: false } }));
    });
  }, [pluginPanels]);

  const closePanel = useCallback(async (label: PanelLabel) => {
    await closePanelWindow(label);
    setPanels((prev) => ({ ...prev, [label]: { isOpen: false } }));
  }, []);

  const togglePanel = useCallback(
    async (label: PanelLabel) => {
      const existing = await WebviewWindow.getByLabel(label);
      if (existing) {
        await closePanel(label);
      } else {
        await openPanel(label);
      }
    },
    [openPanel, closePanel],
  );

  return {
    panels,
    pluginPanels,
    togglePanel,
  };
}
