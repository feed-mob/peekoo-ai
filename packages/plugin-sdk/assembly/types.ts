/**
 * Information about an active schedule timer.
 */
export class ScheduleInfo {
  owner: string = "";
  key: string = "";
  interval_secs: u64 = 0;
  repeat: bool = false;
  time_remaining_secs: u64 = 0;
}

/**
 * A single badge item displayed on the Peek overlay.
 */
export class BadgeItem {
  label: string = "";
  value: string = "";
  icon: string | null = null;
  countdown_secs: u64 = 0;
}
