import {
  peekoo_schedule_cancel,
  peekoo_schedule_get,
  peekoo_schedule_set,
} from "./host";
import { extractBoolField, extractRawField, extractStringField, extractU64Field, quote } from "./json";
import { readString, writeString } from "./memory";
import { ScheduleInfo } from "./types";

/**
 * Create or replace a schedule timer.
 */
export function set(
  key: string,
  interval_secs: u64,
  repeat: bool,
  delay_secs: u64 = 0,
): void {
  let input = "{\"key\":" + quote(key) + ",\"interval_secs\":" + interval_secs.toString() + ",\"repeat\":" + (repeat ? "true" : "false");
  if (delay_secs > 0) {
    input += ",\"delay_secs\":" + delay_secs.toString();
  }
  input += "}";
  peekoo_schedule_set(writeString(input));
}

/**
 * Cancel a schedule timer.
 */
export function cancel(key: string): void {
  const input = "{\"key\":" + quote(key) + "}";
  peekoo_schedule_cancel(writeString(input));
}

/**
 * Get information about a schedule timer.
 * Returns null if no timer with this key exists.
 */
export function get(key: string): ScheduleInfo | null {
  const input = "{\"key\":" + quote(key) + "}";
  const offset = peekoo_schedule_get(writeString(input));
  const response = readString(offset);

  const scheduleJson = extractRawField(response, "schedule");
  if (scheduleJson.length == 0 || scheduleJson == "null") {
    return null;
  }

  const info = new ScheduleInfo();
  info.owner = extractStringField(scheduleJson, "owner");
  info.key = extractStringField(scheduleJson, "key");
  info.interval_secs = extractU64Field(scheduleJson, "interval_secs");
  info.repeat = extractBoolField(scheduleJson, "repeat");
  info.time_remaining_secs = extractU64Field(scheduleJson, "time_remaining_secs");

  return info;
}
