import { peekoo_set_mood } from "./host";
import { quote } from "./json";
import { writeString } from "./memory";

/**
 * Set the sprite mood.
 *
 * `trigger` must be a valid PetReactionTrigger string (e.g.
 * "opencode-working", "opencode-done", "opencode-idle").
 *
 * When `sticky` is true, the mood persists until another mood is set.
 * When false, the mood reverts to idle after a short timeout.
 *
 * Requires the `pet:mood` permission.
 */
export function set(trigger: string, sticky: bool): void {
  const input =
    '{"trigger":' +
    quote(trigger) +
    ',"sticky":' +
    (sticky ? "true" : "false") +
    "}";
  peekoo_set_mood(writeString(input));
}
