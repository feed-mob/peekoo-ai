import { CheckCircle2, Plus, ArrowRight, Tag, Trash2 } from "lucide-react";
import type { TaskEvent } from "@/types/task";

interface ActivityItemProps {
  event: TaskEvent;
}

function formatTime(isoString: string): string {
  const date = new Date(isoString);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMs / 3600000);

  if (diffMins < 1) return "just now";
  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  return date.toLocaleDateString();
}

function getEventIcon(eventType: string) {
  switch (eventType) {
    case "status_changed":
      return <CheckCircle2 size={14} className="text-green-400" />;
    case "created":
      return <Plus size={14} className="text-blue-400" />;
    case "assigned":
      return <ArrowRight size={14} className="text-purple-400" />;
    case "labeled":
    case "unlabeled":
      return <Tag size={14} className="text-yellow-400" />;
    case "deleted":
      return <Trash2 size={14} className="text-red-400" />;
    default:
      return <CheckCircle2 size={14} className="text-text-muted" />;
  }
}

function getEventDescription(event: TaskEvent): string {
  const title = (event.payload as Record<string, unknown>)?.title as string ?? "Unknown task";
  switch (event.event_type) {
    case "created":
      return `Created "${title}"`;
    case "status_changed": {
      const from = (event.payload as Record<string, unknown>)?.from as string ?? "?";
      const to = (event.payload as Record<string, unknown>)?.to as string ?? "?";
      return `${from} → ${to} for "${title}"`;
    }
    case "assigned": {
      const to = (event.payload as Record<string, unknown>)?.to as string ?? "?";
      return `Assigned "${title}" to ${to}`;
    }
    case "labeled": {
      const label = (event.payload as Record<string, unknown>)?.label as string ?? "?";
      return `Added "${label}" to "${title}"`;
    }
    case "unlabeled": {
      const label = (event.payload as Record<string, unknown>)?.label as string ?? "?";
      return `Removed "${label}" from "${title}"`;
    }
    case "deleted":
      return `Deleted "${title}"`;
    default:
      return `${event.event_type} on "${title}"`;
  }
}

export function ActivityItem({ event }: ActivityItemProps) {
  return (
    <div className="flex items-start gap-2.5 py-2">
      <div className="mt-0.5 shrink-0">{getEventIcon(event.event_type)}</div>
      <div className="flex-1 min-w-0">
        <p className="text-xs text-text-primary leading-relaxed">{getEventDescription(event)}</p>
      </div>
      <span className="text-[10px] text-text-muted shrink-0 mt-0.5">{formatTime(event.created_at)}</span>
    </div>
  );
}
