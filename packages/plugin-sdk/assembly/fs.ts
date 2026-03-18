import { peekoo_fs_read, peekoo_fs_read_dir } from "./host";
import {
  extractBoolField,
  extractRawField,
  extractStringField,
  extractU64Field,
  quote,
} from "./json";
import { readString, writeString } from "./memory";
import { FsEntry } from "./types";

export function read(path: string): string | null {
  return readWithOptions(path, 0);
}

export function readTail(path: string, tailBytes: u64): string | null {
  return readWithOptions(path, tailBytes);
}

export function readDir(path: string): FsEntry[] {
  const input = "{\"path\":" + quote(path) + "}";
  const result = readString(peekoo_fs_read_dir(writeString(input)));
  const rawEntries = extractRawField(result, "entries");
  if (rawEntries.length == 0 || rawEntries == "null" || rawEntries == "[]") {
    return [];
  }

  const items = splitTopLevelArrayItems(rawEntries);
  const entries = new Array<FsEntry>();
  for (let i = 0; i < items.length; i++) {
    const item = items[i];
    if (item.length == 0) continue;
    const entry = new FsEntry();
    entry.name = extractStringField(item, "name");
    entry.is_dir = extractBoolField(item, "is_dir");
    const modifiedRaw = extractRawField(item, "modified_secs");
    entry.modified_secs = modifiedRaw == "null" || modifiedRaw.length == 0
      ? 0
      : extractU64Field(item, "modified_secs");
    entries.push(entry);
  }
  return entries;
}

function readWithOptions(path: string, tailBytes: u64): string | null {
  let input = "{\"path\":" + quote(path);
  if (tailBytes > 0) {
    input += ",\"tail_bytes\":" + tailBytes.toString();
  }
  input += "}";

  const result = readString(peekoo_fs_read(writeString(input)));
  const content = extractStringField(result, "content");
  if (content.length == 0 && result.indexOf('"content":null') >= 0) {
    return null;
  }
  return content;
}

function splitTopLevelArrayItems(rawArray: string): string[] {
  if (rawArray.length < 2) {
    return [];
  }

  let content = rawArray;
  if (content.charAt(0) == '[' && content.charAt(content.length - 1) == ']') {
    content = content.substring(1, content.length - 1);
  }

  const items = new Array<string>();
  let start = 0;
  let depth = 0;
  let inString = false;
  let escaped = false;

  for (let i = 0; i < content.length; i++) {
    const ch = content.charAt(i);
    if (inString) {
      if (escaped) {
        escaped = false;
      } else if (ch == "\\") {
        escaped = true;
      } else if (ch == '"') {
        inString = false;
      }
      continue;
    }

    if (ch == '"') {
      inString = true;
      continue;
    }

    if (ch == '{' || ch == '[') {
      depth++;
      continue;
    }

    if (ch == '}' || ch == ']') {
      depth--;
      continue;
    }

    if (ch == ',' && depth == 0) {
      items.push(content.substring(start, i).trim());
      start = i + 1;
    }
  }

  const tail = content.substring(start).trim();
  if (tail.length > 0) {
    items.push(tail);
  }

  return items;
}
