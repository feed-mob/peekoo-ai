import type { TaskStatus } from "@/types/task";
import type { TFunction } from "i18next";
import {
  parseISODate,
  isToday,
  isTomorrow,
} from "./date-helpers";

export const PRIORITY_CONFIG = {
  high: { color: "#E9762B", labelKey: "tasks.priority.high", dotColor: "#E5484D" },
  medium: { color: "#F5C842", labelKey: "tasks.priority.medium", dotColor: "#F5C842" },
  low: { color: "#7B9AC7", labelKey: "tasks.priority.low", dotColor: "#30A46C" },
} as const;

export const STATUS_CONFIG: Record<
  TaskStatus,
  { color: string; labelKey: string; next: TaskStatus }
> = {
  todo: { color: "#7B9AC7", labelKey: "tasks.status.todo", next: "in_progress" },
  in_progress: { color: "#F5C842", labelKey: "tasks.status.in_progress", next: "done" },
  done: { color: "#30A46C", labelKey: "tasks.status.done", next: "todo" },
};

export function getTaskStatusOptions(t: TFunction): { value: TaskStatus; label: string }[] {
  return [
    { value: "todo", label: t(STATUS_CONFIG.todo.labelKey) },
    { value: "in_progress", label: t(STATUS_CONFIG.in_progress.labelKey) },
    { value: "done", label: t(STATUS_CONFIG.done.labelKey) },
  ];
}

export const RECURRENCE_OPTIONS = [
  { value: "__none__", labelKey: "tasks.detail.doesNotRepeat" },
  { value: "FREQ=DAILY", labelKey: "tasks.recurrence.daily" },
  { value: "FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR", labelKey: "tasks.recurrence.weekdays" },
  { value: "FREQ=WEEKLY;BYDAY=MO,WE,FR", labelKey: "tasks.recurrence.monWedFri" },
  { value: "FREQ=WEEKLY;BYDAY=TU,TH", labelKey: "tasks.recurrence.tueThu" },
  { value: "FREQ=WEEKLY", labelKey: "tasks.recurrence.weekly" },
  { value: "FREQ=MONTHLY", labelKey: "tasks.recurrence.monthly" },
];

export function getRecurrenceOptions(t: TFunction): { value: string; label: string }[] {
  return RECURRENCE_OPTIONS.map(opt => ({ value: opt.value, label: t(opt.labelKey) }));
}

export const TIME_OPTIONS = [
  "06:00", "06:30", "07:00", "07:30", "08:00", "08:30",
  "09:00", "09:30", "10:00", "10:30", "11:00", "11:30",
  "12:00", "12:30", "13:00", "13:30", "14:00", "14:30",
  "15:00", "15:30", "16:00", "16:30", "17:00", "17:30",
  "18:00", "18:30", "19:00", "19:30", "20:00", "20:30",
  "21:00", "21:30", "22:00",
];

function formatRecurrenceTime(time: string): string {
  const [h, m] = time.split(":").map(Number);
  if (m === 0) return `${h}:00`;
  return `${h}:${String(m).padStart(2, "0")}`;
}

export function formatRecurrenceDisplay(
  rule: string,
  timeOfDay: string | null,
  t: TFunction
): string {
  const time = timeOfDay ? formatRecurrenceTime(timeOfDay) : "—";

  switch (rule) {
    case "FREQ=DAILY":
      return `${time} ${t("tasks.recurrence.daily").toLowerCase()}`;
    case "FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR":
      return `${time} ${t("tasks.recurrence.weekdays").toLowerCase()}`;
    case "FREQ=WEEKLY;BYDAY=MO,WE,FR":
      return `${time} ${t("tasks.recurrence.monWedFri").toLowerCase()}`;
    case "FREQ=WEEKLY;BYDAY=TU,TH":
      return `${time} ${t("tasks.recurrence.tueThu").toLowerCase()}`;
    case "FREQ=WEEKLY":
      return `${time} ${t("tasks.recurrence.weekly").toLowerCase()}`;
    case "FREQ=MONTHLY":
      return `${time} ${t("tasks.recurrence.monthly").toLowerCase()}`;
    default:
      return `${time} ${rule}`;
  }
}

export function formatTimeRange(
  start: string | null,
  end: string | null,
  recurrenceRule: string | null,
  recurrenceTimeOfDay: string | null,
  t: TFunction
): string | null {
  if (recurrenceRule && recurrenceTimeOfDay) {
    return formatRecurrenceDisplay(recurrenceRule, recurrenceTimeOfDay, t);
  }

  if (!start && !end) return null;

  const fmtTime = (d: Date) =>
    d.toLocaleTimeString("en-US", {
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
    });

  const dayLabel = (d: Date): string => {
    if (isToday(d)) return t("tasks.formatting.today");
    if (isTomorrow(d)) return t("tasks.formatting.tomorrow");
    return d.toLocaleDateString("en-US", { month: "short", day: "numeric" });
  };

  if (!start && end) {
    const endDate = parseISODate(end)!;
    return `${dayLabel(endDate)} ${fmtTime(endDate)}`;
  }

  const startDate = parseISODate(start)!;

  if (end) {
    const endDate = parseISODate(end)!;
    const startDayStr = dayLabel(startDate);
    const endDayStr = dayLabel(endDate);

    if (startDayStr === endDayStr) {
      return `${startDayStr} ${fmtTime(startDate)} – ${fmtTime(endDate)}`;
    }
    return `${startDayStr} ${fmtTime(startDate)} → ${endDayStr} ${fmtTime(endDate)}`;
  }

  return `${dayLabel(startDate)} ${fmtTime(startDate)}`;
}

export function formatDuration(minutes: number, t: TFunction): string {
  if (minutes < 60) return `${minutes}${t("tasks.formatting.minutesShort")}`;
  const hours = Math.floor(minutes / 60);
  const mins = minutes % 60;
  if (mins === 0) return `${hours}${t("tasks.formatting.hoursShort")}`;
  return `${hours}${t("tasks.formatting.hoursShort")} ${mins}${t("tasks.formatting.minutesShort")}`;
}

export function getLabelColor(label: string): string {
  let hash = 0;
  for (let i = 0; i < label.length; i++) {
    hash = label.charCodeAt(i) + ((hash << 5) - hash);
  }
  const hue = Math.abs(hash) % 360;
  return `hsl(${hue}, 60%, 55%)`;
}
