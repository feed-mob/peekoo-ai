import { Host } from "@extism/as-pdk";
import * as badge from "../../../packages/plugin-sdk/assembly/badge";
import * as fs from "../../../packages/plugin-sdk/assembly/fs";
import * as log from "../../../packages/plugin-sdk/assembly/log";
import * as mood from "../../../packages/plugin-sdk/assembly/mood";
import * as notify from "../../../packages/plugin-sdk/assembly/notify";
import * as schedule from "../../../packages/plugin-sdk/assembly/schedule";
import * as state from "../../../packages/plugin-sdk/assembly/state";
import { BadgeItem } from "../../../packages/plugin-sdk/assembly/types";
import { extractRawField, extractStringField, extractU64Field } from "../../../packages/plugin-sdk/assembly/json";

const SCHEDULE_KEY = "poll-claude-code";
// 5 s keeps UI responsive while staying well below ~/.claude JSONL write frequency
const POLL_INTERVAL_SECS: u64 = 5;
const PROJECTS_DIR = "~/.claude/projects";
const TAIL_BYTES: u64 = 16_384;
const STALE_WORKING_POLLS: u64 = 30;
const RECENT_WINDOW_SECS: u64 = 300;
const STATE_LAST_STATUS = "last_status";

class SessionSnapshot {
  sessionId: string = "";
  slug: string = "";
  status: string = "idle";
  modifiedSecs: u64 = 0;
  completionKey: string = "";
}

class AggregateState {
  primarySlug: string = "";
  status: string = "idle";
  completionCount: i32 = 0;
}

export function abort(
  message: string | null,
  fileName: string | null,
  lineNumber: u32,
  columnNumber: u32,
): void {
  const messageText = message === null ? "abort" : changetype<string>(message);
  const fileText = fileName === null ? "unknown" : changetype<string>(fileName);
  log.error(fileText + ":" + lineNumber.toString() + ":" + columnNumber.toString() + ": " + messageText);
}

export function plugin_init(): i32 {
  log.info("Claude Code Companion: initializing");
  schedule.set(SCHEDULE_KEY, POLL_INTERVAL_SECS, true);
  state.set(STATE_LAST_STATUS, "idle");
  badge.set([]);
  Host.outputString('{"status":"ok"}');
  return 0;
}

export function on_event(): i32 {
  const input = Host.inputString();
  const eventName = extractStringField(input, "event");
  if (eventName != "schedule:fired") {
    Host.outputString('{"ok":true}');
    return 0;
  }

  const payload = extractRawField(input, "payload");
  if (extractStringField(payload, "key") != SCHEDULE_KEY) {
    Host.outputString('{"ok":true}');
    return 0;
  }

  pollClaudeCode();
  Host.outputString('{"ok":true}');
  return 0;
}

export function debug_infer_status(): i32 {
  const input = Host.inputString();
  const jsonl = extractStringField(input, "jsonl");
  const modifiedSecs = extractU64Field(input, "modified_secs");
  const unchangedPolls = extractU64Field(input, "unchanged_polls");
  const snapshot = inferSessionSnapshot(jsonl, modifiedSecs, unchangedPolls);
  Host.outputString(snapshotToJson(snapshot));
  return 0;
}

export function debug_aggregate_status(): i32 {
  const raw = Host.inputString().trim();
  const statuses = new Array<string>();
  if (raw.length > 0) {
    const parts = raw.split(",");
    for (let i = 0; i < parts.length; i++) {
      statuses.push(parts[i]);
    }
  }
  let hasWaiting = false;
  let hasWorking = false;
  for (let i = 0; i < statuses.length; i++) {
    const status = statuses[i].trim();
    if (status == "waiting") hasWaiting = true;
    if (status == "working") hasWorking = true;
  }

  const value = hasWaiting ? "waiting" : (hasWorking ? "working" : "idle");
  Host.outputString('{"status":"' + value + '"}');
  return 0;
}

function pollClaudeCode(): void {
  const sessions = discoverSessions();
  const aggregate = aggregateSessions(sessions);
  const previousStatus = state.get(STATE_LAST_STATUS);

  for (let i = 0; i < sessions.length; i++) {
    const session = sessions[i];
    if (session.status == "done" && session.completionKey.length > 0) {
      const completedKeyName = "completed:" + session.sessionId;
      const seenCompletion = state.get(completedKeyName);
      if (seenCompletion != session.completionKey) {
        notify.send("Claude Code", "\u2705 " + displayTitle(session.slug) + " is done!");
        state.set(completedKeyName, session.completionKey);
        aggregate.completionCount += 1;
      }
    }
  }

  if (aggregate.status != previousStatus) {
    handleStatusChange(aggregate.status, aggregate.primarySlug, aggregate.completionCount);
    state.set(STATE_LAST_STATUS, aggregate.status);
  } else if (aggregate.status == "working" || aggregate.status == "waiting") {
    updateBadge(aggregate.status, aggregate.primarySlug);
  }
}

function discoverSessions(): SessionSnapshot[] {
  const sessions = new Array<SessionSnapshot>();
  const projectDirs = fs.readDir(PROJECTS_DIR);

  // First pass: find the globally newest mtime across all projects.
  // This serves as a proxy for "now" since the WASM plugin has no clock.
  let globalNewestMtime: u64 = 0;

  // Collect candidates: one newest JSONL per project dir.
  const candidates = new Array<CandidateSession>();

  for (let i = 0; i < projectDirs.length; i++) {
    const projectDir = projectDirs[i];
    if (!projectDir.is_dir) continue;

    const projectPath = PROJECTS_DIR + "/" + projectDir.name;
    const entries = fs.readDir(projectPath);
    let newestEntryIndex: i32 = -1;
    let newestModified: u64 = 0;
    for (let j = 0; j < entries.length; j++) {
      const entry = entries[j];
      if (entry.is_dir || !entry.name.endsWith(".jsonl")) continue;
      if (newestEntryIndex < 0 || entry.modified_secs >= newestModified) {
        newestModified = entry.modified_secs;
        newestEntryIndex = j;
      }
    }

    if (newestEntryIndex < 0) continue;

    const entry = entries[newestEntryIndex];
    if (entry.modified_secs > globalNewestMtime) {
      globalNewestMtime = entry.modified_secs;
    }
    const candidate = new CandidateSession();
    candidate.path = projectPath + "/" + entry.name;
    candidate.sessionId = entry.name.substring(0, entry.name.length - 6);
    candidate.modifiedSecs = entry.modified_secs;
    candidates.push(candidate);
  }

  // Second pass: only process candidates that are recent relative to the
  // globally newest file. This prevents stale sessions from other projects
  // from showing up alongside active ones.
  for (let i = 0; i < candidates.length; i++) {
    const candidate = candidates[i];
    if (globalNewestMtime > candidate.modifiedSecs &&
        globalNewestMtime - candidate.modifiedSecs > RECENT_WINDOW_SECS) {
      continue;
    }

    const jsonl = fs.readTail(candidate.path, TAIL_BYTES);
    if (jsonl === null) continue;

    const pollState = readPollState(candidate.sessionId, candidate.modifiedSecs);
    const snapshot = inferSessionSnapshot(jsonl as string, candidate.modifiedSecs, pollState[1]);
    if (snapshot.sessionId.length == 0) {
      snapshot.sessionId = candidate.sessionId;
    }
    writePollState(snapshot.sessionId, candidate.modifiedSecs, snapshot.status, pollState[1]);
    if (snapshot.status != "idle" || snapshot.completionKey.length > 0) {
      sessions.push(snapshot);
    }
  }

  return sessions;
}

class CandidateSession {
  path: string = "";
  sessionId: string = "";
  modifiedSecs: u64 = 0;
}

function aggregateSessions(sessions: SessionSnapshot[]): AggregateState {
  const aggregate = new AggregateState();
  let primaryModified: u64 = 0;
  let hasWaiting = false;
  let hasWorking = false;

  for (let i = 0; i < sessions.length; i++) {
    const session = sessions[i];
    if (session.status == "waiting") hasWaiting = true;
    if (session.status == "working") hasWorking = true;
    if ((session.status == "waiting" || session.status == "working") && session.modifiedSecs >= primaryModified) {
      primaryModified = session.modifiedSecs;
      aggregate.primarySlug = session.slug;
    }
  }

  aggregate.status = hasWaiting ? "waiting" : (hasWorking ? "working" : "idle");
  return aggregate;
}

function handleStatusChange(status: string, slug: string, completionCount: i32): void {
  if (status == "working") {
    mood.set("claude-working", true);
    updateBadge(status, slug);
    return;
  }

  if (status == "waiting") {
    mood.set("claude-reminder", false);
    notify.send("Claude Code", "Claude Code needs your input");
    updateBadge(status, slug);
    return;
  }

  if (completionCount > 0) {
    mood.set("claude-done", false);
  } else {
    mood.set("claude-idle", false);
  }
  badge.set([]);
}

function updateBadge(status: string, slug: string): void {
  const item = new BadgeItem();
  item.label = "Claude Code";
  item.value = status == "waiting" ? "Needs input" : displayTitle(slug);
  item.icon = "activity";
  badge.set([item]);
}

function displayTitle(slug: string): string {
  return slug.length > 0 ? slug : "Working...";
}

function inferSessionSnapshot(jsonl: string, modifiedSecs: u64, unchangedPolls: u64): SessionSnapshot {
  const snapshot = new SessionSnapshot();
  snapshot.modifiedSecs = modifiedSecs;

  const lines = jsonl.split("\n");
  for (let i = lines.length - 1; i >= 0; i--) {
    const line = lines[i].trim();
    if (line.length == 0) continue;

    if (snapshot.sessionId.length == 0) {
      snapshot.sessionId = extractStringField(line, "sessionId");
    }
    if (snapshot.slug.length == 0) {
      snapshot.slug = extractStringField(line, "slug");
    }

    const lineType = extractTopLevelType(line);
    if (lineType == "assistant") {
      const message = extractRawField(line, "message");
      const stopReason = extractStringField(message, "stop_reason");
      if (stopReason == "end_turn" || stopReason == "stop_sequence") {
        snapshot.status = "done";
        snapshot.completionKey = snapshot.sessionId;
        return snapshot;
      }

      snapshot.status = unchangedPolls >= STALE_WORKING_POLLS ? "idle" : "working";
      return snapshot;
    }

    if (lineType == "user") {
      if (extractRawField(line, "isMeta") == "true") {
        continue;
      }

      if (isInterruptedUserLine(line)) {
        snapshot.status = unchangedPolls >= STALE_WORKING_POLLS ? "idle" : "waiting";
        return snapshot;
      }

      snapshot.status = unchangedPolls >= STALE_WORKING_POLLS ? "idle" : "working";
      return snapshot;
    }
  }

  return snapshot;
}

function readPollState(sessionId: string, modifiedSecs: u64): u64[] {
  const stored = state.get("poll:" + sessionId);
  if (stored.length == 0) {
    return [modifiedSecs, 0];
  }

  const parts = stored.split("|");
  if (parts.length < 2) {
    return [modifiedSecs, 0];
  }

  const previousModified = U64.parseInt(parts[0]);
  const unchangedPolls = U64.parseInt(parts[1]);
  if (previousModified == modifiedSecs) {
    return [previousModified, unchangedPolls + 1];
  }

  return [modifiedSecs, 0];
}

function writePollState(sessionId: string, modifiedSecs: u64, status: string, unchangedPolls: u64): void {
  const nextUnchanged = status == "working" || status == "waiting" ? unchangedPolls : 0;
  state.set("poll:" + sessionId, modifiedSecs.toString() + "|" + nextUnchanged.toString());
}

function isInterruptedUserLine(line: string): bool {
  return line.indexOf('"text":"[Request interrupted by user]"') >= 0;
}

function snapshotToJson(snapshot: SessionSnapshot): string {
  return "{" +
    '"session_id":"' + escapeJson(snapshot.sessionId) + '",' +
    '"slug":"' + escapeJson(snapshot.slug) + '",' +
    '"status":"' + escapeJson(snapshot.status) + '",' +
    '"modified_secs":' + snapshot.modifiedSecs.toString() + "," +
    '"completion_key":"' + escapeJson(snapshot.completionKey) + '"' +
    "}";
}

/**
 * Extract the top-level "type" field from a JSON line.
 *
 * Claude Code JSONL lines have deeply nested "type" fields inside objects
 * like `message`. The generic `extractStringField` returns the first match,
 * which is often a nested one (e.g. "message" instead of "assistant").
 *
 * This function walks the string tracking brace/bracket depth and only
 * matches `"type":` when depth == 1 (inside the root `{}`).
 */
function extractTopLevelType(json: string): string {
  const marker = '"type":';
  let depth: i32 = 0;
  let inString = false;
  let escaped = false;

  for (let i = 0; i < json.length; i++) {
    const ch = json.charAt(i);

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
      // Check if this is the start of "type": at root depth
      if (depth == 1 && i + marker.length <= json.length) {
        const candidate = json.substring(i, i + marker.length);
        if (candidate == marker) {
          // Found "type": at root — extract the string value
          let valueStart = i + marker.length;
          // Skip whitespace
          while (valueStart < json.length && (json.charAt(valueStart) == ' ' || json.charAt(valueStart) == '\n')) {
            valueStart++;
          }
          if (valueStart < json.length && json.charAt(valueStart) == '"') {
            let value = "";
            for (let j = valueStart + 1; j < json.length; j++) {
              const vc = json.charAt(j);
              if (vc == '"') {
                return value;
              }
              value += vc;
            }
          }
          return "";
        }
      }
      inString = true;
      continue;
    }

    if (ch == '{' || ch == '[') {
      depth++;
    } else if (ch == '}' || ch == ']') {
      depth--;
    }
  }

  return "";
}

function escapeJson(value: string): string {
  let escaped = "";
  for (let i = 0; i < value.length; i++) {
    const ch = value.charAt(i);
    if (ch == "\\") {
      escaped += "\\\\";
    } else if (ch == '"') {
      escaped += '\\"';
    } else {
      escaped += ch;
    }
  }
  return escaped;
}
