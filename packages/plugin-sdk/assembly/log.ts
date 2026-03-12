import { peekoo_log } from "./host";
import { quote } from "./json";
import { writeString } from "./memory";

function log(level: string, message: string): void {
  const input = "{\"level\":" + quote(level) + ",\"message\":" + quote(message) + "}";
  peekoo_log(writeString(input));
}

/** Log an info-level message. */
export function info(message: string): void {
  log("info", message);
}

/** Log a warning-level message. */
export function warn(message: string): void {
  log("warn", message);
}

/** Log an error-level message. */
export function error(message: string): void {
  log("error", message);
}

/** Log a debug-level message. */
export function debug(message: string): void {
  log("debug", message);
}
