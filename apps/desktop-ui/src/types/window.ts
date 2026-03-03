import { z } from "zod";

export const WindowLabelSchema = z.enum([
  "main",
  "panel-chat",
  "panel-tasks",
  "panel-pomodoro",
]);
export type WindowLabel = z.infer<typeof WindowLabelSchema>;

export const PanelLabelSchema = z.enum([
  "panel-chat",
  "panel-tasks",
  "panel-pomodoro",
]);
export type PanelLabel = z.infer<typeof PanelLabelSchema>;

export const PANEL_LABELS = PanelLabelSchema.options;

export const PanelWindowConfigSchema = z.object({
  label: PanelLabelSchema,
  title: z.string(),
  width: z.number(),
  height: z.number(),
});
export type PanelWindowConfig = z.infer<typeof PanelWindowConfigSchema>;

export const PANEL_WINDOW_CONFIGS: Record<PanelLabel, PanelWindowConfig> = {
  "panel-chat": {
    label: "panel-chat",
    title: "Peekoo Chat",
    width: 420,
    height: 600,
  },
  "panel-tasks": {
    label: "panel-tasks",
    title: "Peekoo Tasks",
    width: 340,
    height: 420,
  },
  "panel-pomodoro": {
    label: "panel-pomodoro",
    title: "Peekoo Pomodoro",
    width: 300,
    height: 380,
  },
};
