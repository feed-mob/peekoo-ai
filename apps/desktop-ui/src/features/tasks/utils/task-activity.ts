import { KNOWN_AGENTS, type TaskEvent } from "@/types/task";
import type { TFunction } from "i18next";

export const TASKS_CHANGED_EVENT = "tasks-changed";

function getCommentAuthor(event: TaskEvent): string {
  const payload = event.payload as Record<string, unknown>;
  return (payload?.author as string) ?? "user";
}

export function getCommentAuthorDisplayName(event: TaskEvent, t: TFunction): string {
  const author = getCommentAuthor(event);

  if (author === "user") {
    return t("tasks.activity.you");
  }

  if (author === "agent") {
    return t("tasks.activity.agent");
  }

  const knownAgent = KNOWN_AGENTS.find((candidate) => candidate.id === author);
  return knownAgent?.name ?? author;
}

export function isUserComment(event: TaskEvent): boolean {
  return getCommentAuthor(event) === "user";
}

export function getCommentText(event: TaskEvent): string {
  const payload = event.payload as Record<string, unknown>;
  return (payload?.text as string) ?? "";
}
