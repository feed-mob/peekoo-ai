import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

function createElement(tagName = "div") {
  const element = {
    tagName,
    className: "",
    textContent: "",
    disabled: false,
    children: [],
    listeners: new Map(),
    append(...nodes) {
      this.children.push(...nodes);
    },
    appendChild(node) {
      this.children.push(node);
      return node;
    },
    addEventListener(type, listener) {
      this.listeners.set(type, listener);
    },
  };

  Object.defineProperty(element, "innerHTML", {
    get() {
      return "";
    },
    set() {
      this.children = [];
    },
  });

  return element;
}

function createStatusPayload() {
  return {
    pomodoro_active: false,
    reminders: [
      {
        reminder_type: "water",
        interval_min: 45,
        time_remaining_secs: 1200,
        active: true,
      },
    ],
  };
}

async function loadPanelScript() {
  const summary = createElement("section");
  const status = createElement("section");
  const refreshButton = createElement("button");
  const intervalCalls = [];
  const invokeCalls = [];
  let now = 0;

  globalThis.document = {
    getElementById(id) {
      if (id === "summary") return summary;
      if (id === "status") return status;
      if (id === "refreshButton") return refreshButton;
      return null;
    },
    createElement,
  };

  globalThis.window = {
    __TAURI__: {
      core: {
        async invoke(command, payload) {
          invokeCalls.push({ command, payload });
          return JSON.stringify(createStatusPayload());
        },
      },
    },
  };

  globalThis.console = console;
  Date.now = () => now;
  globalThis.setInterval = (callback, delay) => {
    intervalCalls.push({ callback, delay });
    return intervalCalls.length;
  };
  globalThis.clearInterval = () => {};

  const script = readFileSync(
    resolve(import.meta.dir, "../../../plugins/health-reminders/ui/panel.js"),
    "utf8",
  );

  new Function(script)();
  await Promise.resolve();
  await Promise.resolve();

  return {
    intervalCalls,
    invokeCalls,
    advanceTime(milliseconds) {
      now += milliseconds;
    },
    readNextDueText() {
      return status.children[0]?.children[0]?.children[2]?.textContent ?? null;
    },
  };
}

describe("health reminders panel", () => {
  test("starts periodic status polling after the initial refresh", async () => {
    const { intervalCalls, invokeCalls } = await loadPanelScript();

    expect(invokeCalls).toHaveLength(1);
    expect(intervalCalls.some((call) => call.delay === 30000)).toBe(true);
  });

  test("updates the rendered countdown between backend polls", async () => {
    const { intervalCalls, advanceTime, readNextDueText } = await loadPanelScript();

    expect(readNextDueText()).toBe("Next in ~20 min");

    const countdownTick = intervalCalls.find((call) => call.delay === 1000);

    expect(countdownTick).toBeDefined();

    advanceTime(61000);
    countdownTick.callback();

    expect(readNextDueText()).toBe("Next in ~19 min");
  });

  test("refreshes backend immediately when a reminder becomes due", async () => {
    const { intervalCalls, advanceTime, invokeCalls } = await loadPanelScript();
    const countdownTick = intervalCalls.find((call) => call.delay === 1000);

    advanceTime(20 * 60 * 1000);
    await countdownTick.callback();
    await Promise.resolve();

    expect(invokeCalls).toHaveLength(2);
  });
});
