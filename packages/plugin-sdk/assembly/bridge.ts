import { peekoo_bridge_fs_read } from "./host";
import { extractStringField } from "./json";
import { writeString, readString } from "./memory";

/**
 * Read the bridge file for this plugin.
 *
 * Returns the file contents as a string, or null if the file does not exist.
 * The file path is platform-specific and always scoped to the current plugin
 * key, which is injected by the host and cannot be overridden.
 *
 * Requires the `bridge:fs_read` permission.
 */
export function read(): string | null {
  const result = readString(peekoo_bridge_fs_read(writeString("")));
  const content = extractStringField(result, "content");
  // The host returns {"content":null} when the file does not exist.
  // extractStringField returns "" for null values.
  if (content.length == 0) {
    // Double-check: distinguish between empty-string content and null.
    if (result.indexOf('"content":null') >= 0) {
      return null;
    }
  }
  return content;
}
