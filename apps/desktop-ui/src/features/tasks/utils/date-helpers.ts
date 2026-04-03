/**
 * Date helper utilities for reliable date handling
 * Never use string slicing for date comparison!
 */

/**
 * Parse ISO string to Date object (safely)
 */
export function parseISODate(isoString: string | null | undefined): Date | null {
  if (!isoString) return null;
  const date = new Date(isoString);
  if (isNaN(date.getTime())) return null;
  return date;
}

/**
 * Get date string in YYYY-MM-DD format (for comparison)
 */
export function toDateString(date: Date | string): string {
  const d = typeof date === "string" ? new Date(date) : date;
  const year = d.getFullYear();
  const month = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

/**
 * Check if two dates are the same calendar day
 */
export function isSameDay(date1: Date | string, date2: Date | string): boolean {
  return toDateString(date1) === toDateString(date2);
}

/**
 * Check if date is today
 */
export function isToday(date: Date | string): boolean {
  return isSameDay(date, new Date());
}

/**
 * Check if date is tomorrow
 */
export function isTomorrow(date: Date | string): boolean {
  const tomorrow = new Date();
  tomorrow.setDate(tomorrow.getDate() + 1);
  return isSameDay(date, tomorrow);
}

/**
 * Check if date is within the next 7 days (including today)
 */
export function isThisWeek(date: Date | string): boolean {
  const d = typeof date === "string" ? new Date(date) : date;
  const now = new Date();
  const weekFromNow = new Date(now.getTime() + 7 * 24 * 60 * 60 * 1000);
  return d >= now && d <= weekFromNow;
}

/**
 * Compare two dates for sorting
 * Returns negative if a < b, positive if a > b, 0 if equal
 */
export function compareDates(
  a: string | null | undefined,
  b: string | null | undefined
): number {
  const dateA = parseISODate(a);
  const dateB = parseISODate(b);

  if (!dateA && !dateB) return 0;
  if (!dateA) return 1; // nulls last
  if (!dateB) return -1; // nulls last

  return dateA.getTime() - dateB.getTime();
}

/**
 * Format relative time (e.g., "just now", "5m ago", "2h ago")
 */
import type { TFunction } from "i18next";

export function formatRelativeTime(isoString: string, t?: TFunction): string {
  const date = parseISODate(isoString);
  if (!date) return "";

  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMs / 3600000);
  const diffDays = Math.floor(diffMs / 86400000);

  if (diffMins < 1) return t ? t("dateHelpers.justNow", "just now") : "just now";
  if (diffMins < 60) return t ? `${diffMins}${t("tasks.formatting.minutesShort")} ${t("dateHelpers.ago", "ago")}` : `${diffMins}m ago`;
  if (diffHours < 24) return t ? `${diffHours}${t("tasks.formatting.hoursShort")} ${t("dateHelpers.ago", "ago")}` : `${diffHours}h ago`;
  if (diffDays === 1) return t ? t("tasks.activity.yesterday") : "yesterday";
  if (diffDays < 7) return t ? `${diffDays} ${t("dateHelpers.daysAgo", "days ago")}` : `${diffDays} days ago`;

  return date.toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
  });
}

/**
 * Format time from ISO string (e.g., "14:30")
 */
export function formatTime(isoString: string): string {
  const date = parseISODate(isoString);
  if (!date) return "";

  return date.toLocaleTimeString("en-US", {
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  });
}

/**
 * Convert date/time to ISO string for backend
 */
export function fromDateTimeLocal(
  date: string | undefined,
  time: string | undefined
): string | null {
  if (!date || !time) return null;
  return new Date(`${date}T${time}:00`).toISOString();
}

/**
 * Extract date portion from ISO string for date input
 */
export function toDateInputValue(isoString: string | null): string {
  const date = parseISODate(isoString);
  if (!date) return "";
  return toDateString(date);
}

/**
 * Extract time portion from ISO string for time input
 */
export function toTimeInputValue(isoString: string | null): string {
  const date = parseISODate(isoString);
  if (!date) return "";
  const hours = String(date.getHours()).padStart(2, "0");
  const minutes = String(date.getMinutes()).padStart(2, "0");
  return `${hours}:${minutes}`;
}

/**
 * Check if a task is overdue (start time in past and not done)
 */
export function isOverdue(
  startAt: string | null,
  status: string
): boolean {
  if (!startAt || status === "done") return false;
  const start = parseISODate(startAt);
  if (!start) return false;
  return start.getTime() < Date.now();
}
