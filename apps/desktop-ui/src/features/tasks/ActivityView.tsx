import { useMemo } from "react";
import type { TaskEvent } from "@/types/task";
import { ActivityItem } from "./ActivityItem";

interface ActivityViewProps {
  events: TaskEvent[];
}

function groupByDay(events: TaskEvent[]): { label: string; events: TaskEvent[] }[] {
  const today = new Date().toISOString().slice(0, 10);
  const yesterday = new Date(Date.now() - 86400000).toISOString().slice(0, 10);

  const groups: Record<string, TaskEvent[]> = {};
  for (const event of events) {
    const day = event.created_at.slice(0, 10);
    if (!groups[day]) groups[day] = [];
    groups[day].push(event);
  }

  const result: { label: string; events: TaskEvent[] }[] = [];
  const sortedDays = Object.keys(groups).sort().reverse();

  for (const day of sortedDays) {
    let label = day;
    if (day === today) label = "Today";
    else if (day === yesterday) label = "Yesterday";
    result.push({ label, events: groups[day] });
  }

  return result;
}

export function ActivityView({ events }: ActivityViewProps) {
  const grouped = useMemo(() => groupByDay(events), [events]);

  if (events.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-12 text-center">
        <p className="text-sm text-text-muted">No activity yet</p>
        <p className="text-xs text-text-muted/60 mt-1">Task events will appear here</p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {grouped.map((group) => (
        <div key={group.label}>
          <h3 className="text-[10px] font-semibold text-text-muted uppercase tracking-wider mb-1">
            {group.label}
          </h3>
          <div className="divide-y divide-glass-border/50">
            {group.events.map((event) => (
              <ActivityItem key={event.id} event={event} />
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}
