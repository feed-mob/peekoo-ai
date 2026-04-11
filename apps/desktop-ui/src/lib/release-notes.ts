const LEADING_HTML_COMMENT_PATTERN = /^\s*<!--[\s\S]*?-->\s*/;

export function normalizeReleaseNotes(notes?: string | null): string | null {
  if (!notes) {
    return null;
  }

  const normalized = notes
    .replace(/\r\n/g, "\n")
    .replace(LEADING_HTML_COMMENT_PATTERN, "")
    .trim();

  if (!normalized) {
    return null;
  }

  return normalized;
}
