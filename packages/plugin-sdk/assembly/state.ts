import { peekoo_state_get, peekoo_state_set } from "./host";
import { extractRawField, extractStringField, quote } from "./json";
import { readString, writeString } from "./memory";

/**
 * Get a string value from plugin state.
 * Returns an empty string if the key does not exist.
 */
export function get(key: string): string {
  const input = "{\"key\":" + quote(key) + "}";
  const offset = peekoo_state_get(writeString(input));
  const response = readString(offset);
  const raw = extractRawField(response, "value");
  if (raw == "null") {
    return "";
  }
  if (raw.length >= 2 && raw.charAt(0) == '"' && raw.charAt(raw.length - 1) == '"') {
    return extractStringField(response, "value");
  }
  return raw;
}

/**
 * Set a string value in plugin state.
 */
export function set(key: string, value: string): void {
  const input = "{\"key\":" + quote(key) + ",\"value\":" + quote(value) + "}";
  peekoo_state_set(writeString(input));
}

/**
 * Delete a key from plugin state (sets it to null).
 */
export function del(key: string): void {
  const input = "{\"key\":" + quote(key) + ",\"value\":null}";
  peekoo_state_set(writeString(input));
}
