import { useMemo } from "react";
import type { TaskEvent } from "@/types/task";
import { ActivityFeedItem } from "./ActivityFeedItem";
import { LoadingSpinner } from "./LoadingSpinner";
import { useTranslation } from "react-i18next";

interface ActivityFeedProps {
  events: TaskEvent[];
  isLoading?: boolean;
}

function groupByDay(
  events: TaskEvent[],
  labels: { today: string; yesterday: string },
): { label: string; events: TaskEvent[] }[] {
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
    if (day === today) label = labels.today;
    else if (day === yesterday) label = labels.yesterday;
    result.push({ label, events: groups[day] });
  }

  return result;
}

export function ActivityFeed({ events, isLoading = false }: ActivityFeedProps) {
  const { t } = useTranslation();
  const grouped = useMemo(
    () =>
      groupByDay(events, {
        today: t("tasks.tabs.today"),
        yesterday: t("tasks.activity.yesterday"),
      }),
    [events, t],
  );

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <LoadingSpinner />
      </div>
    );
  }

  if (events.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-12 text-center">
        <p className="text-sm text-text-muted">{t("tasks.activity.noActivity")}</p>
        <p className="text-xs text-text-muted/60 mt-1">
          {t("tasks.activity.eventsAppearHere")}
        </p>
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
              <ActivityFeedItem key={event.id} event={event} />
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}
