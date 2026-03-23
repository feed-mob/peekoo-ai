import type { Task, TaskTab } from "@/types/task";
import { parseISODate, isToday, isThisWeek } from "./date-helpers";

/**
 * Filter tasks based on the active tab
 */
export function filterTasksByTab(
  tasks: Task[],
  tab: TaskTab,
  _today: Date,
  _weekEnd: Date
): Task[] {
  switch (tab) {
    case "today":
      return tasks.filter((t) => {
        if (t.status === "done") return false;
        // Include tasks scheduled for today OR unscheduled tasks
        if (!t.scheduled_start_at) return true;
        const start = parseISODate(t.scheduled_start_at);
        if (!start) return true;
        return isToday(start);
      });

    case "week":
      return tasks.filter((t) => {
        if (t.status === "done") return false;
        if (!t.scheduled_start_at) return false;
        const start = parseISODate(t.scheduled_start_at);
        if (!start) return false;
        // This week but not today (those are in "today" tab)
        return isThisWeek(start) && !isToday(start);
      });

    case "all":
      return tasks.filter((t) => t.status !== "done");

    case "done":
      return tasks.filter((t) => t.status === "done");

    default:
      return tasks;
  }
}

/**
 * Sort tasks by:
 * 1. Scheduled tasks by start time (ascending)
 * 2. Unscheduled tasks by creation time
 */
export function sortTasks(tasks: Task[]): Task[] {
  return [...tasks].sort((a, b) => {
    // Both have scheduled start - sort by time
    if (a.scheduled_start_at && b.scheduled_start_at) {
      return a.scheduled_start_at.localeCompare(b.scheduled_start_at);
    }

    // Scheduled tasks come before unscheduled
    if (a.scheduled_start_at) return -1;
    if (b.scheduled_start_at) return 1;

    // Both unscheduled - sort by creation time
    return (a.created_at || "").localeCompare(b.created_at || "");
  });
}
