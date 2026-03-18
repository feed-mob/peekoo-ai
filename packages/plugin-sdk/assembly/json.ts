export function quote(value: string): string {
  let escaped = "";
  for (let i = 0; i < value.length; i++) {
    const ch = value.charAt(i);
    if (ch == "\\") {
      escaped += "\\\\";
    } else if (ch == '"') {
      escaped += '\\"';
    } else if (ch == "\n") {
      escaped += "\\n";
    } else if (ch == "\r") {
      escaped += "\\r";
    } else if (ch == "\t") {
      escaped += "\\t";
    } else {
      escaped += ch;
    }
  }
  return '"' + escaped + '"';
}

export function extractStringField(json: string, field: string): string {
  const marker = '"' + field + '":';
  const markerIndex = json.indexOf(marker);
  if (markerIndex < 0) {
    return "";
  }

  const start = json.indexOf('"', markerIndex + marker.length);
  if (start < 0) {
    return "";
  }

  let value = "";
  let escaped = false;
  for (let i = start + 1; i < json.length; i++) {
    const ch = json.charAt(i);
    if (escaped) {
      if (ch == 'n') {
        value += "\n";
      } else if (ch == 'r') {
        value += "\r";
      } else if (ch == 't') {
        value += "\t";
      } else {
        value += ch;
      }
      escaped = false;
      continue;
    }

    if (ch == "\\") {
      escaped = true;
      continue;
    }

    if (ch == '"') {
      return value;
    }

    value += ch;
  }

  return value;
}

export function extractRawField(json: string, field: string): string {
  const marker = '"' + field + '":';
  const markerIndex = json.indexOf(marker);
  if (markerIndex < 0) {
    return "";
  }

  let start = markerIndex + marker.length;
  while (start < json.length && (json.charAt(start) == ' ' || json.charAt(start) == '\n')) {
    start++;
  }

  let end = start;
  let depth = 0;
  let inString = false;
  let escaped = false;
  for (; end < json.length; end++) {
    const ch = json.charAt(end);
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
      if (depth == 0) {
        break;
      }
      depth--;
      continue;
    }

    if (depth == 0 && ch == ',') {
      break;
    }
  }

  return json.substring(start, end).trim();
}

export function extractU64Field(json: string, field: string): u64 {
  const raw = extractRawField(json, field);
  return raw.length == 0 ? 0 : U64.parseInt(raw);
}

export function extractBoolField(json: string, field: string): bool {
  return extractRawField(json, field) == "true";
}

export function extractIntField(json: string, field: string): i32 {
  let raw = extractRawField(json, field);
  if (raw.length == 0) return 0;
  if (raw.length >= 2 && raw.charAt(0) == '"' && raw.charAt(raw.length - 1) == '"') {
    raw = raw.substring(1, raw.length - 1);
  }
  if (raw.length == 0) return 0;

  let result: i32 = 0;
  let negative: bool = false;
  let start: i32 = 0;

  if (raw.charAt(0) == '-') {
    negative = true;
    start = 1;
  }

  for (let i: i32 = start; i < raw.length; i++) {
    const c: i32 = raw.charCodeAt(i);
    if (c >= 48 && c <= 57) {
      result = result * 10 + (c - 48);
    } else {
      break;
    }
  }

  return negative ? -result : result;
}
