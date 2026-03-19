import { z } from "zod";

export const BUILTIN_PANEL_LABELS = [
  "panel-chat",
  "panel-tasks",
  "panel-pomodoro",
  "panel-plugins",
  "panel-settings",
  "panel-about",
  "panel-pomodoro-memo",
] as const;

export const WindowLabelSchema = z.string().refine(
  (value) => value === "main" || value.startsWith("panel-"),
  { message: "Expected main or a panel-* window label" },
);
export type WindowLabel = z.infer<typeof WindowLabelSchema>;

export const PanelLabelSchema = z
  .string()
  .refine((value) => value.startsWith("panel-"), {
    message: "Expected a panel-* label",
  });
export type PanelLabel = z.infer<typeof PanelLabelSchema>;

export const PanelWindowConfigSchema = z.object({
  label: PanelLabelSchema,
  title: z.string(),
  width: z.number(),
  height: z.number(),
});
export type PanelWindowConfig = z.infer<typeof PanelWindowConfigSchema>;

export const PANEL_WINDOW_CONFIGS: Record<string, PanelWindowConfig> = {
  "panel-chat": {
    label: "panel-chat",
    title: "Peekoo Chat",
    width: 520,
    height: 640,
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
    height: 350,
  },
  "panel-plugins": {
    label: "panel-plugins",
    title: "Peekoo Plugins",
    width: 500,
    height: 600,
  },
  "panel-settings": {
    label: "panel-settings",
    title: "Peekoo Settings",
    width: 420,
    height: 500,
  },
  "panel-about": {
    label: "panel-about",
    title: "About Peekoo",
    width: 420,
    height: 440,
  },
  "panel-pomodoro-memo": {
    label: "panel-pomodoro-memo",
    title: "Focus Memo",
    width: 360,
    height: 440,
  },
};
