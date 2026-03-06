import { useState, useCallback } from "react";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { LogicalSize } from "@tauri-apps/api/dpi";
import type { PanelLabel } from "@/types/window";
import { PANEL_WINDOW_CONFIGS, PANEL_LABELS } from "@/types/window";
import { emitPetReaction } from "@/lib/pet-events";

interface PanelWindowState {
  isOpen: boolean;
}

export type PanelWindowStates = Record<PanelLabel, PanelWindowState>;

const INITIAL_STATE: PanelWindowStates = {
  "panel-chat": { isOpen: false },
  "panel-tasks": { isOpen: false },
  "panel-pomodoro": { isOpen: false },
};

const SPRITE_SIZE = { width: 200, height: 250 };
const MENU_SIZE = { width: 300, height: 350 };
const PANEL_OFFSET_X = 20;

export function usePanelWindows() {
  const [panels, setPanels] = useState<PanelWindowStates>(INITIAL_STATE);

  const openPanel = useCallback(async (label: PanelLabel) => {
    const config = PANEL_WINDOW_CONFIGS[label];

    // Check if window already exists
    const existing = await WebviewWindow.getByLabel(label);
    if (existing) {
      await existing.setFocus();
      setPanels((prev) => ({ ...prev, [label]: { isOpen: true } }));
      void emitPetReaction("panel-opened");
      return;
    }

    // Get sprite window position for relative placement
    const spriteWindow = getCurrentWindow();
    const spritePos = await spriteWindow.outerPosition();
    const spriteSize = await spriteWindow.outerSize();

    const panelX = spritePos.x + spriteSize.width + PANEL_OFFSET_X;
    const panelY = spritePos.y;

    const webview = new WebviewWindow(label, {
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
  }, []);

  const closePanel = useCallback(async (label: PanelLabel) => {
    const existing = await WebviewWindow.getByLabel(label);
    if (existing) {
      await existing.close();
    }
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

  const expandForMenu = useCallback(async () => {
    const win = getCurrentWindow();
    await win.setSize(new LogicalSize(MENU_SIZE.width, MENU_SIZE.height));
  }, []);

  const shrinkToSprite = useCallback(async () => {
    const win = getCurrentWindow();
    await win.setSize(new LogicalSize(SPRITE_SIZE.width, SPRITE_SIZE.height));
  }, []);

  const hasAnyOpen = PANEL_LABELS.some((label) => panels[label].isOpen);

  return {
    panels,
    openPanel,
    closePanel,
    togglePanel,
    expandForMenu,
    shrinkToSprite,
    hasAnyOpen,
  };
}
