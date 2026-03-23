import type { TaskStatus } from "@/types/task";
import {
  parseISODate,
  isToday,
  isTomorrow,
} from "./date-helpers";

/**
 * Priority configuration with colors and labels
 */
export const PRIORITY_CONFIG = {
  high: { color: "#E9762B", label: "High", dotColor: "#E5484D" },
  medium: { color: "#F5C842", label: "Medium", dotColor: "#F5C842" },
  low: { color: "#7B9AC7", label: "Low", dotColor: "#30A46C" },
} as const;

/**
 * Status configuration with colors, labels, and next status for cycling
 */
export const STATUS_CONFIG: Record<
  TaskStatus,
  { color: string; label: string; next: TaskStatus }
> = {
  todo: { color: "#7B9AC7", label: "Todo", next: "in_progress" },
  in_progress: { color: "#F5C842", label: "In Progress", next: "done" },
  done: { color: "#30A46C", label: "Done", next: "todo" },
};

/**
 * Recurrence rule options for dropdown
 */
export const RECURRENCE_OPTIONS = [
  { value: "__none__", label: "Does not repeat" },
  { value: "FREQ=DAILY", label: "Daily" },
  { value: "FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR", label: "Every weekday" },
  { value: "FREQ=WEEKLY;BYDAY=MO,WE,FR", label: "Mon / Wed / Fri" },
  { value: "FREQ=WEEKLY;BYDAY=TU,TH", label: "Tue / Thu" },
  { value: "FREQ=WEEKLY", label: "Weekly" },
  { value: "FREQ=MONTHLY", label: "Monthly" },
];

/**
 * Time options for recurring task time picker
 */
export const TIME_OPTIONS = [
  "06:00", "06:30", "07:00", "07:30", "08:00", "08:30",
  "09:00", "09:30", "10:00", "10:30", "11:00", "11:30",
  "12:00", "12:30", "13:00", "13:30", "14:00", "14:30",
  "15:00", "15:30", "16:00", "16:30", "17:00", "17:30",
  "18:00", "18:30", "19:00", "19:30", "20:00", "20:30",
  "21:00", "21:30", "22:00",
];

/**
 * Format recurrence time (e.g., "9:00" or "9:30")
 */
function formatRecurrenceTime(time: string): string {
  const [h, m] = time.split(":").map(Number);
  if (m === 0) return `${h}:00`;
  return `${h}:${String(m).padStart(2, "0")}`;
}

/**
 * Format recurrence display (e.g., "9:00 daily", "9:00 Mon/Wed/Fri")
 */
export function formatRecurrenceDisplay(
  rule: string,
  timeOfDay: string | null
): string {
  const time = timeOfDay ? formatRecurrenceTime(timeOfDay) : "—";

  switch (rule) {
    case "FREQ=DAILY":
      return `${time} daily`;
    case "FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR":
      return `${time} weekdays`;
    case "FREQ=WEEKLY;BYDAY=MO,WE,FR":
      return `${time} Mon/Wed/Fri`;
    case "FREQ=WEEKLY;BYDAY=TU,TH":
      return `${time} Tue/Thu`;
    case "FREQ=WEEKLY":
      return `${time} weekly`;
    case "FREQ=MONTHLY":
      return `${time} monthly`;
    default:
      return `${time} ${rule}`;
  }
}

/**
 * Format time range for display in task list
 * Handles both scheduled tasks and recurring tasks
 */
export function formatTimeRange(
  start: string | null,
  end: string | null,
  recurrenceRule: string | null,
  recurrenceTimeOfDay: string | null
): string | null {
  // Recurring tasks: show recurrence pattern
  if (recurrenceRule && recurrenceTimeOfDay) {
    return formatRecurrenceDisplay(recurrenceRule, recurrenceTimeOfDay);
  }

  // No schedule
  if (!start && !end) return null;

  // Format helpers
  const fmtTime = (d: Date) =>
    d.toLocaleTimeString("en-US", {
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
    });

  const dayLabel = (d: Date): string => {
    if (isToday(d)) return "Today";
    if (isTomorrow(d)) return "Tomorrow";
    return d.toLocaleDateString("en-US", { month: "short", day: "numeric" });
  };

  // Only end time
  if (!start && end) {
    const endDate = parseISODate(end)!;
    return `${dayLabel(endDate)} ${fmtTime(endDate)}`;
  }

  // Has start time
  const startDate = parseISODate(start)!;

  // Start and end
  if (end) {
    const endDate = parseISODate(end)!;
    const startDayStr = dayLabel(startDate);
    const endDayStr = dayLabel(endDate);

    if (startDayStr === endDayStr) {
      return `${startDayStr} ${fmtTime(startDate)} – ${fmtTime(endDate)}`;
    }
    return `${startDayStr} ${fmtTime(startDate)} → ${endDayStr} ${fmtTime(endDate)}`;
  }

  // Only start time
  return `${dayLabel(startDate)} ${fmtTime(startDate)}`;
}

/**
 * Format duration in minutes to human readable string
 */
export function formatDuration(minutes: number): string {
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  const mins = minutes % 60;
  if (mins === 0) return `${hours}h`;
  return `${hours}h ${mins}m`;
}

/**
 * Get label color for custom labels (hash-based)
 */
export function getLabelColor(label: string): string {
  let hash = 0;
  for (let i = 0; i < label.length; i++) {
    hash = label.charCodeAt(i) + ((hash << 5) - hash);
  }
  const hue = Math.abs(hash) % 360;
  return `hsl(${hue}, 60%, 55%)`;
}
