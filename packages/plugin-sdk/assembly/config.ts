import { peekoo_config_get } from "./host";
import { extractRawField, extractStringField, quote } from "./json";
import { readString, writeString } from "./memory";

/**
 * Get a single configuration value as a string.
 * Returns an empty string if the key does not exist.
 */
export function get(key: string): string {
  const input = "{\"key\":" + quote(key) + "}";
  const offset = peekoo_config_get(writeString(input));
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
 * Get all configuration values as a raw JSON string.
 */
export function getAll(): string {
  const input = "{}";
  const offset = peekoo_config_get(writeString(input));
  const response = readString(offset);
  const raw = extractRawField(response, "value");
  return raw == "null" ? "{}" : raw;
}
