import type { Task } from "@/types/task";
import { isOverdue, parseISODate, toDateString } from "./date-helpers";

function wasFinishedToday(task: Task, todayKey: string): boolean {
  if (task.status !== "done" || !task.finished_at) {
    return false;
  }

  const finishedAt = parseISODate(task.finished_at);
  if (!finishedAt) {
    return false;
  }

  return toDateString(finishedAt) === todayKey;
}

export function splitTodayTasks(
  tasks: Task[],
  today = new Date()
): {
  overdue: Task[];
  today: Task[];
  unscheduled: Task[];
  completed: Task[];
} {
  const todayKey = toDateString(today);

  return {
    overdue: tasks.filter(
      (task) =>
        task.status !== "done" &&
        !!task.scheduled_start_at &&
        isOverdue(task.scheduled_start_at, task.status, today)
    ),
    today: tasks.filter((task) => {
      if (task.status === "done" || !task.scheduled_start_at) return false;
      const start = parseISODate(task.scheduled_start_at);
      if (!start) return false;
      return toDateString(start) === todayKey;
    }),
    unscheduled: tasks.filter(
      (task) => task.status !== "done" && !task.scheduled_start_at
    ),
    completed: tasks.filter((task) => wasFinishedToday(task, todayKey)),
  };
}
