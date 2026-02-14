import type { LucideIcon } from "lucide-react";

export type PanelId = "chat" | "tasks" | "pomodoro";

export interface PanelPosition {
  x: number;
  y: number;
}

export interface PanelConfig {
  id: PanelId;
  title: string;
  icon: LucideIcon;
  width: number;
  height: number;
  defaultPosition: PanelPosition;
}

export interface PanelState {
  isOpen: boolean;
  isMinimized: boolean;
}

export type PanelStates = Record<PanelId, PanelState>;
