import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";

import createPlugin from "@extism/extism";

const pluginDir = path.resolve(import.meta.dirname, "..");
const wasmPath = path.join(pluginDir, "build", "peekoo_claude_code_companion.wasm");
const fixturesDir = path.join(import.meta.dirname, "fixtures");

async function fixture(name) {
  return readFile(path.join(fixturesDir, name), "utf8");
}

function jsonResponse(callContext, value) {
  return callContext.store(JSON.stringify(value));
}

function createMockHost({ directories = {}, files = {} } = {}) {
  const stateStore = new Map();
  const schedules = new Map();
  const notifications = [];
  const badges = [];
  const moods = [];

  const functions = {
    env: {
      abort(callContext, messageOffset, fileNameOffset, lineNumber, columnNumber) {
        const message = callContext.read(messageOffset)?.text() ?? "abort";
        const fileName = callContext.read(fileNameOffset)?.text() ?? "unknown";
        throw new Error(`${fileName}:${lineNumber}:${columnNumber}: ${message}`);
      },
    },
    "extism:host/user": {
      peekoo_state_get(callContext, offs) {
        const req = JSON.parse(callContext.read(offs).text());
        return jsonResponse(callContext, {
          value: stateStore.has(req.key) ? stateStore.get(req.key) : null,
        });
      },
      peekoo_state_set(callContext, offs) {
        const req = JSON.parse(callContext.read(offs).text());
        if (req.value === null) {
          stateStore.delete(req.key);
        } else {
          stateStore.set(req.key, req.value);
        }
        return jsonResponse(callContext, { ok: true });
      },
      peekoo_log(callContext, offs) {
        void offs;
        return jsonResponse(callContext, { ok: true });
      },
      peekoo_notify(callContext, offs) {
        notifications.push(JSON.parse(callContext.read(offs).text()));
        return jsonResponse(callContext, { ok: true, suppressed: false });
      },
      peekoo_schedule_set(callContext, offs) {
        const req = JSON.parse(callContext.read(offs).text());
        schedules.set(req.key, req);
        return jsonResponse(callContext, { ok: true });
      },
      peekoo_schedule_cancel(callContext, offs) {
        const req = JSON.parse(callContext.read(offs).text());
        schedules.delete(req.key);
        return jsonResponse(callContext, { ok: true });
      },
      peekoo_schedule_get(callContext, offs) {
        const req = JSON.parse(callContext.read(offs).text());
        return jsonResponse(callContext, {
          schedule: schedules.get(req.key) ?? null,
        });
      },
      peekoo_set_peek_badge(callContext, offs) {
        badges.push(JSON.parse(callContext.read(offs).text()));
        return jsonResponse(callContext, { ok: true });
      },
      peekoo_fs_read(callContext, offs) {
        const req = JSON.parse(callContext.read(offs).text());
        const full = files[req.path] ?? null;
        let content = full;
        if (full != null && req.tail_bytes != null) {
          content = full.slice(-req.tail_bytes);
        }
        return jsonResponse(callContext, { content });
      },
      peekoo_fs_read_dir(callContext, offs) {
        const req = JSON.parse(callContext.read(offs).text());
        return jsonResponse(callContext, { entries: directories[req.path] ?? [] });
      },
      peekoo_set_mood(callContext, offs) {
        moods.push(JSON.parse(callContext.read(offs).text()));
        return jsonResponse(callContext, { ok: true });
      },
    },
  };

  return { functions, stateStore, schedules, notifications, badges, moods };
}

async function withPlugin(host, run) {
  const plugin = await createPlugin(wasmPath, {
    useWasi: true,
    functions: host.functions,
  });

  try {
    return await run(plugin);
  } finally {
    await plugin.close();
  }
}

test("debug_infer_status detects working tool_use tails", async () => {
  const host = createMockHost();
  const input = JSON.stringify({
    jsonl: await fixture("working-tool-use.jsonl"),
    modified_secs: 100,
    unchanged_polls: 0,
  });

  await withPlugin(host, async (plugin) => {
    const output = await plugin.call("debug_infer_status", input);
    const parsed = output.json();
    assert.equal(parsed.status, "working");
    assert.equal(parsed.slug, "rapid-silver-comet");
    assert.equal(parsed.session_id, "session-working");
  });
});

test("debug_infer_status detects done sessions and completion keys", async () => {
  const host = createMockHost();
  const input = JSON.stringify({
    jsonl: await fixture("done-end-turn.jsonl"),
    modified_secs: 222,
    unchanged_polls: 0,
  });

  await withPlugin(host, async (plugin) => {
    const output = await plugin.call("debug_infer_status", input);
    const parsed = output.json();
    assert.equal(parsed.status, "done");
    assert.equal(parsed.slug, "steady-mint-river");
    assert.equal(parsed.completion_key, "session-done:222");
  });
});

test("plugin_init installs the polling schedule", async () => {
  const host = createMockHost();

  await withPlugin(host, async (plugin) => {
    const output = await plugin.call("plugin_init", "");
    assert.deepEqual(output.json(), { status: "ok" });
  });

  assert.equal(host.schedules.get("poll-claude-code").interval_secs, 5);
});

test("on_event sends completion notification once for a done session", async () => {
  const doneJsonl = await fixture("done-end-turn.jsonl");
  const host = createMockHost({
    directories: {
      "~/.claude/projects": [{ name: "-repo", is_dir: true, modified_secs: 400 }],
      "~/.claude/projects/-repo": [{ name: "session-done.jsonl", is_dir: false, modified_secs: 400 }],
    },
    files: {
      "~/.claude/projects/-repo/session-done.jsonl": doneJsonl,
    },
  });

  await withPlugin(host, async (plugin) => {
    await plugin.call("plugin_init", "");
    const input = JSON.stringify({ event: "schedule:fired", payload: { key: "poll-claude-code" } });
    await plugin.call("on_event", input);
    await plugin.call("on_event", input);
  });

  assert.equal(host.notifications.length, 1);
  assert.match(host.notifications[0].body, /steady-mint-river is done!/);
});

test("on_event marks long-stale working sessions idle", async () => {
  const workingJsonl = await fixture("working-tool-use.jsonl");
  const host = createMockHost({
    directories: {
      "~/.claude/projects": [{ name: "-repo", is_dir: true, modified_secs: 500 }],
      "~/.claude/projects/-repo": [{ name: "session-working.jsonl", is_dir: false, modified_secs: 500 }],
    },
    files: {
      "~/.claude/projects/-repo/session-working.jsonl": workingJsonl,
    },
  });

  await withPlugin(host, async (plugin) => {
    await plugin.call("plugin_init", "");
    const input = JSON.stringify({ event: "schedule:fired", payload: { key: "poll-claude-code" } });
    for (let i = 0; i < 31; i++) {
      await plugin.call("on_event", input);
    }
  });

  assert.equal(host.stateStore.get("last_status"), "idle");
  assert.ok(host.moods.some((entry) => entry.trigger === "claude-idle"));
});

test("on_event prefers the newest session in a project over older waiting sessions", async () => {
  const waitingJsonl = await fixture("waiting-user.jsonl");
  const workingJsonl = await fixture("working-tool-use.jsonl");
  const host = createMockHost({
    directories: {
      "~/.claude/projects": [{ name: "-repo", is_dir: true, modified_secs: 610 }],
      "~/.claude/projects/-repo": [
        { name: "session-waiting.jsonl", is_dir: false, modified_secs: 600 },
        { name: "session-working.jsonl", is_dir: false, modified_secs: 610 },
      ],
    },
    files: {
      "~/.claude/projects/-repo/session-waiting.jsonl": waitingJsonl,
      "~/.claude/projects/-repo/session-working.jsonl": workingJsonl,
    },
  });

  await withPlugin(host, async (plugin) => {
    await plugin.call("plugin_init", "");
    const input = JSON.stringify({ event: "schedule:fired", payload: { key: "poll-claude-code" } });
    await plugin.call("on_event", input);
  });

  assert.equal(host.stateStore.get("last_status"), "working");
  assert.deepEqual(host.badges.at(-1), [{ label: "Claude Code", value: "rapid-silver-comet", icon: "activity" }]);
  assert.ok(host.moods.some((entry) => entry.trigger === "claude-working"));
});

test("on_event ignores old projects when a recent project exists", async () => {
  const waitingJsonl = await fixture("waiting-user.jsonl");
  const workingJsonl = await fixture("working-tool-use.jsonl");
  const now = 100000;
  const host = createMockHost({
    directories: {
      "~/.claude/projects": [
        { name: "-old-project", is_dir: true, modified_secs: now - 86400 },
        { name: "-active-project", is_dir: true, modified_secs: now },
      ],
      "~/.claude/projects/-old-project": [
        { name: "old-session.jsonl", is_dir: false, modified_secs: now - 86400 },
      ],
      "~/.claude/projects/-active-project": [
        { name: "active-session.jsonl", is_dir: false, modified_secs: now },
      ],
    },
    files: {
      "~/.claude/projects/-old-project/old-session.jsonl": waitingJsonl,
      "~/.claude/projects/-active-project/active-session.jsonl": workingJsonl,
    },
  });

  await withPlugin(host, async (plugin) => {
    await plugin.call("plugin_init", "");
    const input = JSON.stringify({ event: "schedule:fired", payload: { key: "poll-claude-code" } });
    await plugin.call("on_event", input);
  });

  assert.equal(host.stateStore.get("last_status"), "working");
  assert.ok(host.moods.some((entry) => entry.trigger === "claude-working"));
  // No "waiting" notification from the old project
  assert.equal(host.notifications.length, 0);
});

test("on_event clears stale waiting sessions after enough unchanged polls", async () => {
  const waitingJsonl = await fixture("waiting-user.jsonl");
  const host = createMockHost({
    directories: {
      "~/.claude/projects": [{ name: "-repo", is_dir: true, modified_secs: 700 }],
      "~/.claude/projects/-repo": [{ name: "session-waiting.jsonl", is_dir: false, modified_secs: 700 }],
    },
    files: {
      "~/.claude/projects/-repo/session-waiting.jsonl": waitingJsonl,
    },
  });

  await withPlugin(host, async (plugin) => {
    await plugin.call("plugin_init", "");
    const input = JSON.stringify({ event: "schedule:fired", payload: { key: "poll-claude-code" } });
    for (let i = 0; i < 31; i++) {
      await plugin.call("on_event", input);
    }
  });

  assert.equal(host.stateStore.get("last_status"), "idle");
  assert.deepEqual(host.badges.at(-1), []);
  assert.ok(host.moods.some((entry) => entry.trigger === "claude-idle"));
});
