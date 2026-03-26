import type { SpriteBubblePayload } from "@/types/sprite-bubble";

export const DEFAULT_SPRITE_BUBBLE_DURATION_MS = 5000;
export const PRIORITY_SPRITE_BUBBLE_DURATION_MS = 10000;

export type SpriteBubbleKind =
  | "water"
  | "eye"
  | "stand"
  | "focus"
  | "break"
  | "task"
  | "calendar"
  | "default";

export function getSpriteBubbleDurationMs(payload: SpriteBubblePayload): number {
  return isPriorityNotification(payload)
    ? PRIORITY_SPRITE_BUBBLE_DURATION_MS
    : DEFAULT_SPRITE_BUBBLE_DURATION_MS;
}

export function getSpriteBubbleKind(payload: SpriteBubblePayload): SpriteBubbleKind {
  const sourcePlugin = payload.sourcePlugin?.toLowerCase();
  const bodyLower = payload.body.toLowerCase();
  const titleLower = payload.title.toLowerCase();

  if (sourcePlugin === "tasks") {
    return "task";
  }

  if (sourcePlugin === "google-calendar") {
    return "calendar";
  }

  const isHealth = titleLower.includes("health");
  if (isHealth) {
    if (bodyLower.includes("water")) return "water";
    if (bodyLower.includes("eye")) return "eye";
    if (bodyLower.includes("stand")) return "stand";
  }

  if (titleLower.includes("focus")) {
    return "focus";
  }

  if (titleLower.includes("break")) {
    return "break";
  }

  return "default";
}

function isPriorityNotification(payload: SpriteBubblePayload): boolean {
  const sourcePlugin = payload.sourcePlugin?.toLowerCase();
  return sourcePlugin === "tasks" || sourcePlugin === "google-calendar";
}
