import { peekoo_emit_event } from "./host";
import { quote } from "./json";
import { writeString } from "./memory";

/**
 * Emit a named event with a JSON payload string.
 */
export function emit(event: string, payload: string = "{}"): void {
  const input = "{\"event\":" + quote(event) + ",\"payload\":" + payload + "}";
  peekoo_emit_event(writeString(input));
}
