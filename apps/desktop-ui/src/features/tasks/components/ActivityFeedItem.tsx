import { CheckCircle2, Plus, ArrowRight, Tag, Trash2, User, MessageCircle } from "lucide-react";
import { Streamdown } from "streamdown";
import type { TaskEvent } from "@/types/task";
import { formatRelativeTime } from "../utils/date-helpers";
import {
  getCommentAuthorDisplayName,
  getCommentText,
  isUserComment,
} from "../utils/task-activity";
import { useTranslation } from "react-i18next";

interface ActivityFeedItemProps {
  event: TaskEvent;
  compact?: boolean;
  onDelete?: (eventId: string) => void;
  isDeleting?: boolean;
}

const eventIcons = {
  created: Plus,
  status_changed: CheckCircle2,
  assigned: ArrowRight,
  labeled: Tag,
  unlabeled: Tag,
  deleted: Trash2,
  updated: User,
  comment: MessageCircle,
};

const eventColors = {
  created: "text-blue-700 dark:text-blue-400",
  status_changed: "text-green-700 dark:text-green-400",
  assigned: "text-purple-700 dark:text-purple-400",
  labeled: "text-yellow-700 dark:text-yellow-400",
  unlabeled: "text-orange-700 dark:text-orange-400",
  deleted: "text-red-700 dark:text-red-400",
  updated: "text-text-muted",
  comment: "text-cyan-700 dark:text-cyan-400",
};

function getEventDescription(event: TaskEvent, compact: boolean, t: (key: string, vars?: Record<string, string>) => string): string {
  const payload = event.payload as Record<string, unknown>;
  const title = (payload?.title as string) ?? t("tasks.activityFeed.unknownTask");

  switch (event.event_type) {
    case "created":
      return compact
        ? t("tasks.activityFeed.created")
        : t("tasks.activityFeed.createdFull", { title });

    case "status_changed": {
      const from = (payload?.from as string) ?? "?";
      const to = (payload?.to as string) ?? "?";
      return compact
        ? `${from} → ${to}`
        : t("tasks.activityFeed.statusChangedFull", { from, to, title });
    }

    case "assigned": {
      const to = (payload?.to as string) ?? "?";
      return compact
        ? `${t("tasks.activityFeed.assigned")} → ${to}`
        : t("tasks.activityFeed.assignedFull", { title, to });
    }

    case "labeled": {
      const label = (payload?.label as string) ?? "?";
      return t("tasks.activityFeed.labeledFull", { label, title });
    }

    case "unlabeled": {
      const label = (payload?.label as string) ?? "?";
      return t("tasks.activityFeed.unlabeledFull", { label, title });
    }

    case "deleted":
      return compact
        ? t("tasks.activityFeed.deleted")
        : t("tasks.activityFeed.deletedFull", { title });

    case "updated":
      return compact
        ? t("tasks.activityFeed.updated")
        : t("tasks.activityFeed.updatedFull", { title });

    case "comment":
      return compact
        ? t("tasks.activityFeed.comment")
        : t("tasks.activityFeed.commentFull");

    default:
      return compact
        ? event.event_type
        : `${event.event_type} on "${title}"`;
  }
}

function getEventLabel(event: TaskEvent, t: (key: string) => string): string {
  switch (event.event_type) {
    case "created":
      return t("tasks.activityFeed.created");
    case "status_changed":
      return t("tasks.activityFeed.statusChanged");
    case "assigned":
      return t("tasks.activityFeed.assigned");
    case "labeled":
      return t("tasks.activityFeed.labeled");
    case "unlabeled":
      return t("tasks.activityFeed.unlabeled");
    case "deleted":
      return t("tasks.activityFeed.deleted");
    case "updated":
      return t("tasks.activityFeed.updated");
    case "comment":
      return t("tasks.activityFeed.comment");
    default:
      return event.event_type;
  }
}

function getEventTitle(event: TaskEvent): string | null {
  const payload = event.payload as Record<string, unknown>;
  return (payload?.title as string) ?? null;
}

function isCommentEvent(event: TaskEvent): boolean {
  return event.event_type === "comment";
}

export function ActivityFeedItem({ event, compact = false, onDelete, isDeleting = false }: ActivityFeedItemProps) {
  const { t } = useTranslation();
  const Icon = eventIcons[event.event_type as keyof typeof eventIcons] ?? User;
  const colorClass = eventColors[event.event_type as keyof typeof eventColors] ?? "text-text-muted";
  const eventLabel = getEventLabel(event, t);
  const eventTitle = getEventTitle(event);

  if (isCommentEvent(event)) {
    const authorLabel = getCommentAuthorDisplayName(event);
    const text = getCommentText(event);
    const isUser = isUserComment(event);

    if (compact) {
      return (
        <div className={`flex items-start gap-2 py-1.5 ${isUser ? "flex-row-reverse" : ""}`}>
          {isUser && onDelete && (
            <button
              onClick={() => onDelete(event.id)}
              disabled={isDeleting}
              className="opacity-0 group-hover:opacity-100 p-0.5 rounded hover:bg-color-danger/10 text-text-muted hover:text-color-danger transition-all disabled:opacity-50"
              aria-label={t("tasks.activityFeed.deleteComment")}
            >
              <Trash2 size={10} />
            </button>
          )}
          <div className="flex flex-col gap-0.5 max-w-[85%]">
            <span className={`text-[9px] font-medium ${isUser ? "text-right text-green-700 dark:text-green-400" : "text-left text-purple-700 dark:text-purple-400"}`}>
              {authorLabel}
            </span>
              <div
              className={`px-2.5 py-1.5 rounded-lg text-xs ${
                isUser
                  ? "bg-space-deep border border-glass-border text-text-primary"
                  : "bg-purple-500/15 border border-purple-500/30 text-purple-900 dark:bg-purple-500/20 dark:text-purple-100"
              }`}
            >
              <Streamdown>{text}</Streamdown>
            </div>
          </div>
          <span className="text-[10px] text-text-muted shrink-0">
            {formatRelativeTime(event.created_at, t)}
          </span>
        </div>
      );
    }

    return (
      <div className={`flex items-start gap-2 py-2 group ${isUser ? "flex-row-reverse" : ""}`}>
        {isUser && onDelete && (
          <button
            onClick={() => onDelete(event.id)}
            disabled={isDeleting}
            className="mt-1 opacity-0 group-hover:opacity-100 p-1 rounded hover:bg-color-danger/10 text-text-muted hover:text-color-danger transition-all disabled:opacity-50 shrink-0"
            aria-label={t("tasks.activityFeed.deleteComment")}
          >
            <Trash2 size={12} />
          </button>
        )}
        <div className={`flex flex-col ${isUser ? "items-end" : "items-start"}`}>
          <span className={`text-[9px] font-medium mb-0.5 ${isUser ? "text-green-700 dark:text-green-400" : "text-purple-700 dark:text-purple-400"}`}>
            {authorLabel}
          </span>
          <div
            className={`max-w-[80%] px-3 py-2 rounded-xl text-sm ${
              isUser
                ? "bg-space-deep border border-glass-border text-text-primary rounded-br-sm"
                : "bg-purple-500/15 border border-purple-500/30 text-purple-900 rounded-bl-sm dark:bg-purple-500/20 dark:text-purple-100"
            }`}
          >
            <Streamdown>{text}</Streamdown>
          </div>
          <span className="text-[10px] text-text-muted mt-1">
            {formatRelativeTime(event.created_at, t)}
          </span>
        </div>
      </div>
    );
  }

  if (compact) {
    return (
      <div className="flex items-center gap-2 py-1.5">
        <Icon size={12} className={colorClass} />
        <span
          className={`shrink-0 rounded-full border px-1.5 py-0.5 text-[9px] font-semibold ${colorClass}`}
          style={{ borderColor: "currentColor" }}
        >
          {eventLabel}
        </span>
        <span className="flex-1 text-xs text-text-primary truncate">
          {getEventDescription(event, true, t)}
        </span>
        <span className="text-[10px] text-text-muted">
          {formatRelativeTime(event.created_at, t)}
        </span>
      </div>
    );
  }

  return (
    <div className="flex items-start gap-2.5 py-2">
      <div className="mt-0.5 shrink-0 rounded-full bg-space-deep/80 p-1.5 border border-glass-border">
        <Icon size={14} className={colorClass} />
      </div>
      <div className="flex-1 min-w-0 rounded-lg border border-glass-border/60 bg-space-deep/40 px-3 py-2">
        <div className="mb-1 flex items-center gap-2">
          <span
            className={`rounded-full border px-1.5 py-0.5 text-[9px] font-semibold ${colorClass}`}
            style={{ borderColor: "currentColor" }}
          >
            {eventLabel}
          </span>
          {eventTitle && (
            <span className="truncate text-[10px] text-text-muted">
              {eventTitle}
            </span>
          )}
        </div>
        <p className="text-xs text-text-primary leading-relaxed">
          {getEventDescription(event, false, t)}
        </p>
      </div>
      <span className="text-[10px] text-text-muted shrink-0 mt-0.5">
        {formatRelativeTime(event.created_at, t)}
      </span>
    </div>
  );
}
