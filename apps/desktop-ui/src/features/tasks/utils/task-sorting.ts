import type { Task, TaskTab } from "@/types/task";
import { parseISODate, isOverdue, isThisWeek, toDateString } from "./date-helpers";

/**
 * Filter tasks based on the active tab
 */
export function filterTasksByTab(
  tasks: Task[],
  tab: TaskTab,
  today: Date,
  _weekEnd: Date
): Task[] {
  const todayKey = toDateString(today);

  switch (tab) {
    case "today":
      return tasks.filter((t) => {
        if (t.status === "done") {
          const finishedAt = parseISODate(t.finished_at);
          return !!finishedAt && toDateString(finishedAt) === todayKey;
        }

        if (!t.scheduled_start_at) return true;
        const start = parseISODate(t.scheduled_start_at);
        if (!start) return true;
        return (
          toDateString(start) === todayKey ||
          isOverdue(t.scheduled_start_at, t.status, today)
        );
      });

    case "week":
      return tasks.filter((t) => {
        if (t.status === "done") return false;
        if (!t.scheduled_start_at) return false;
        const start = parseISODate(t.scheduled_start_at);
        if (!start) return false;
        // This week but not today (those are in "today" tab)
        return isThisWeek(start) && toDateString(start) !== todayKey;
      });

    case "all":
      return tasks.filter((t) => t.status !== "done");

    case "done":
      return tasks.filter((t) => t.status === "done");

    default:
      return tasks;
  }
}

function compareDateDesc(
  left: string | null | undefined,
  right: string | null | undefined
): number {
  const leftDate = parseISODate(left);
  const rightDate = parseISODate(right);

  if (leftDate && rightDate) {
    return rightDate.getTime() - leftDate.getTime();
  }

  if (leftDate) return -1;
  if (rightDate) return 1;
  return 0;
}

/**
 * Sort tasks by:
 * 1. Scheduled tasks by start time (ascending)
 * 2. Unscheduled tasks by creation time
 */
export function sortTasks(tasks: Task[], tab?: TaskTab): Task[] {
  return [...tasks].sort((a, b) => {
    if (tab === "done") {
      const finishedComparison = compareDateDesc(a.finished_at, b.finished_at);
      if (finishedComparison !== 0) {
        return finishedComparison;
      }

      const updatedComparison = compareDateDesc(a.updated_at, b.updated_at);
      if (updatedComparison !== 0) {
        return updatedComparison;
      }

      return (b.created_at || "").localeCompare(a.created_at || "");
    }

    if (tab === "today") {
      if (a.status === "done" && b.status !== "done") return 1;
      if (a.status !== "done" && b.status === "done") return -1;
    }

    // Both have scheduled start - sort by time
    if (a.scheduled_start_at && b.scheduled_start_at) {
      return a.scheduled_start_at.localeCompare(b.scheduled_start_at);
    }

    // Scheduled tasks come before unscheduled
    if (a.scheduled_start_at) return -1;
    if (b.scheduled_start_at) return 1;

    // Both unscheduled - newest first
    return (b.created_at || "").localeCompare(a.created_at || "");
  });
}
