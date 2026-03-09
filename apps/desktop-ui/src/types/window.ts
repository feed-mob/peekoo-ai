import { z } from "zod";

export const BUILTIN_PANEL_LABELS = [
  "panel-chat",
  "panel-tasks",
  "panel-pomodoro",
  "panel-plugins",
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
  "panel-plugins": {
    label: "panel-plugins",
    title: "Peekoo Plugins",
    width: 420,
    height: 560,
  },
};
