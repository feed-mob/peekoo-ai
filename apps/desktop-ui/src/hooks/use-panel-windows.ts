import { useState, useCallback } from "react";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { getCurrentWindow, monitorFromPoint } from "@tauri-apps/api/window";
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
  "panel-settings": { isOpen: false },
  "panel-updater": { isOpen: false },
};

const PANEL_OFFSET_X = 20;
const PANEL_SCREEN_MARGIN = 16;

interface WorkAreaBounds {
  position: { x: number; y: number };
  size: { width: number; height: number };
}

interface PanelPlacementInput {
  spriteX: number;
  spriteY: number;
  spriteWidth: number;
  panelWidth: number;
  panelHeight: number;
  workArea?: WorkAreaBounds;
}

function clampToRange(value: number, min: number, max: number): number {
  if (max < min) {
    return min;
  }
  return Math.min(Math.max(value, min), max);
}

export function calculatePanelPosition({
  spriteX,
  spriteY,
  spriteWidth,
  panelWidth,
  panelHeight,
  workArea,
}: PanelPlacementInput): { x: number; y: number } {
  if (!workArea) {
    return {
      x: spriteX + spriteWidth + PANEL_OFFSET_X,
      y: spriteY,
    };
  }

  const minX = workArea.position.x + PANEL_SCREEN_MARGIN;
  const maxX =
    workArea.position.x + workArea.size.width - panelWidth - PANEL_SCREEN_MARGIN;
  const minY = workArea.position.y + PANEL_SCREEN_MARGIN;
  const maxY =
    workArea.position.y + workArea.size.height - panelHeight - PANEL_SCREEN_MARGIN;

  const preferredRightX = spriteX + spriteWidth + PANEL_OFFSET_X;
  const preferredLeftX = spriteX - panelWidth - PANEL_OFFSET_X;

  const x =
    preferredRightX <= maxX
      ? preferredRightX
      : preferredLeftX >= minX
        ? preferredLeftX
        : clampToRange(preferredRightX, minX, maxX);

  return {
    x,
    y: clampToRange(spriteY, minY, maxY),
  };
}

function resolvePanelConfig(
  label: string,
  pluginPanels: PluginPanel[],
): PanelWindowConfig | PluginPanel | undefined {
  if (PANEL_WINDOW_CONFIGS[label]) {
    return PANEL_WINDOW_CONFIGS[label];
  }
  return pluginPanels.find((panel) => panel.label === label);
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
  const scaleFactor = await spriteWindow.scaleFactor();
  const spritePos = await spriteWindow.outerPosition();
  const spriteSize = await spriteWindow.outerSize();
  const spritePosLogical = spritePos.toLogical(scaleFactor);
  const spriteSizeLogical = spriteSize.toLogical(scaleFactor);
  const monitor = await monitorFromPoint(spritePos.x, spritePos.y);
  const workArea = monitor
    ? {
        position: monitor.workArea.position.toLogical(scaleFactor),
        size: monitor.workArea.size.toLogical(scaleFactor),
      }
    : undefined;
  const { x: panelX, y: panelY } = calculatePanelPosition({
    spriteX: spritePosLogical.x,
    spriteY: spritePosLogical.y,
    spriteWidth: spriteSizeLogical.width,
    panelWidth: config.width,
    panelHeight: config.height,
    workArea,
  });

  return new WebviewWindow(label, {
    url: "/",
    title: config.title,
    width: config.width,
    height: config.height,
    x: panelX,
    y: panelY,
    decorations: false,
    shadow: false,
    transparent: true,
    alwaysOnTop: true,
    skipTaskbar: true,
    resizable: true,
  });
}

export async function closePanelWindow(label: PanelLabel): Promise<void> {
  const existing = await WebviewWindow.getByLabel(label);
  if (existing) {
    await existing.close();
  }
}

export function usePanelWindows() {
  const { plugins: installedPlugins, panels: pluginPanels } = usePlugins();
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
    installedPlugins,
    openPanel,
    togglePanel,
  };
}
