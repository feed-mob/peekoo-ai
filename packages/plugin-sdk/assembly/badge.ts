import { peekoo_set_peek_badge } from "./host";
import { quote } from "./json";
import { writeString } from "./memory";
import { BadgeItem } from "./types";

/**
 * Replace all badge items for this plugin.
 * Pass an empty array to clear the badge.
 */
export function set(items: BadgeItem[]): void {
  // Manually build JSON array since AS JSON support is limited
  let json = "[";
  for (let i = 0; i < items.length; i++) {
    if (i > 0) json += ",";
    const item = items[i];
    json += "{\"label\":" + quote(item.label) + ",\"value\":" + quote(item.value);
    if (item.icon !== null) {
      json += ",\"icon\":" + quote(changetype<string>(item.icon));
    }
    if (item.countdown_secs > 0) {
      json += ",\"countdown_secs\":" + item.countdown_secs.toString();
    }
    json += "}";
  }
  json += "]";
  peekoo_set_peek_badge(writeString(json));
}
