import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

function createClassList() {
  const values = new Set();
  return {
    add(...tokens) {
      tokens.forEach((token) => values.add(token));
    },
    remove(...tokens) {
      tokens.forEach((token) => values.delete(token));
    },
    toggle(token, force) {
      if (force === undefined) {
        if (values.has(token)) {
          values.delete(token);
          return false;
        }
        values.add(token);
        return true;
      }

      if (force) {
        values.add(token);
        return true;
      }

      values.delete(token);
      return false;
    },
    contains(token) {
      return values.has(token);
    },
  };
}

function createElement(tagName = "div") {
  return {
    tagName,
    textContent: "",
    checked: false,
    value: "0",
    disabled: false,
    onclick: null,
    onchange: null,
    oninput: null,
    style: {},
    classList: createClassList(),
  };
}

function createStatusPayload() {
  return {
    water: {
      interval_min: 45,
      time_remaining_secs: 1200,
    },
    eye_rest: {
      interval_min: 20,
      time_remaining_secs: 600,
    },
    standup: {
      interval_min: 60,
      time_remaining_secs: 1800,
    },
    config: {
      global_enabled: true,
      water_interval_min: 45,
      water_enabled: true,
      eye_interval_min: 20,
      eye_enabled: true,
      eye_rest_interval_min: 20,
      eye_rest_enabled: true,
      stand_interval_min: 60,
      stand_enabled: true,
      standup_interval_min: 60,
      standup_enabled: true,
    },
  };
}

async function loadPanelScript() {
  const elements = Object.fromEntries(
    [
      "btnMuteAll",
      "iconMuted",
      "iconActive",
      "cardWater",
      "timeWater",
      "pbWater",
      "stateWater",
      "toggleWater",
      "rangeWater",
      "valWater",
      "resetWater",
      "cardEye",
      "timeEye",
      "pbEye",
      "stateEye",
      "toggleEye",
      "rangeEye",
      "valEye",
      "resetEye",
      "cardStand",
      "timeStand",
      "pbStand",
      "stateStand",
      "toggleStand",
      "rangeStand",
      "valStand",
      "resetStand",
    ].map((id) => [id, createElement()]),
  );

  const intervalCalls = [];
  const invokeCalls = [];

  globalThis.document = {
    getElementById(id) {
      return elements[id] ?? null;
    },
  };

  globalThis.window = {
    __TAURI__: {
      core: {
        async invoke(command, payload) {
          invokeCalls.push({ command, payload });
          return createStatusPayload();
        },
      },
    },
  };

  globalThis.console = console;
  globalThis.setInterval = (callback, delay) => {
    intervalCalls.push({ callback, delay });
    return intervalCalls.length;
  };
  globalThis.clearInterval = () => {};
  globalThis.setTimeout = async (callback) => {
    await callback();
    return 1;
  };
  globalThis.clearTimeout = () => {};

  const script = readFileSync(
    resolve(import.meta.dir, "../../../plugins/health-reminders/ui/panel.js"),
    "utf8",
  );

  new Function(script)();
  await Promise.resolve();
  await Promise.resolve();

  return {
    elements,
    intervalCalls,
    invokeCalls,
  };
}

describe("health reminders panel", () => {
  test("starts periodic status polling after the initial refresh", async () => {
    const { intervalCalls, invokeCalls } = await loadPanelScript();

    expect(invokeCalls).toHaveLength(1);
    expect(intervalCalls.some((call) => call.delay === 1000)).toBe(true);
  });

  test("renders the active reminder countdown from backend status", async () => {
    const { elements } = await loadPanelScript();

    expect(elements.timeWater.textContent).toBe("20:00");
    expect(elements.stateWater.textContent).toBe("ACTIVE TRACKING");
    expect(elements.pbWater.style.width).toBe("56%");
  });

  test("refreshes backend immediately when the global mute button is clicked", async () => {
    const { elements, invokeCalls } = await loadPanelScript();

    await elements.btnMuteAll.onclick();
    await Promise.resolve();

    expect(invokeCalls).toHaveLength(3);
    expect(invokeCalls[1]).toEqual({
      command: "plugin_call_tool",
      payload: {
        toolName: "health_configure",
        argsJson: JSON.stringify({ global_enabled: false }),
      },
    });
    expect(invokeCalls[2]).toEqual({
      command: "plugin_call_tool",
      payload: {
        toolName: "health_get_status",
        argsJson: "{}",
      },
    });
  });
});
