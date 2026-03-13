import { peekoo_notify } from "./host";
import { extractBoolField, quote } from "./json";
import { readString, writeString } from "./memory";

/**
 * Send a desktop notification.
 * Returns true if delivered, false if suppressed (e.g. by DND).
 */
export function send(title: string, body: string): bool {
  const input = "{\"title\":" + quote(title) + ",\"body\":" + quote(body) + "}";
  const offset = peekoo_notify(writeString(input));
  const response = readString(offset);
  return !extractBoolField(response, "suppressed");
}
