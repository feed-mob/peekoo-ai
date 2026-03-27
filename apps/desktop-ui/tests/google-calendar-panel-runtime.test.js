import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

function createElement(tagName = "div") {
  const classes = new Set();
  const element = {
    tagName,
    textContent: "",
    disabled: false,
    value: "",
    files: null,
    children: [],
    listeners: new Map(),
    attributes: new Map(),
    classList: {
      add(...names) {
        names.forEach((name) => classes.add(name));
      },
      remove(...names) {
        names.forEach((name) => classes.delete(name));
      },
      toggle(name, force) {
        if (force === undefined) {
          if (classes.has(name)) {
            classes.delete(name);
            return false;
          }
          classes.add(name);
          return true;
        }
        if (force) {
          classes.add(name);
          return true;
        }
        classes.delete(name);
        return false;
      },
      contains(name) {
        return classes.has(name);
      },
    },
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
    setAttribute(name, value) {
      this.attributes.set(name, value);
    },
    click() {
      const listener = this.listeners.get("click");
      if (listener) {
        listener();
      }
    },
  };

  Object.defineProperty(element, "className", {
    get() {
      return [...classes].join(" ");
    },
    set(value) {
      classes.clear();
      value
        .split(/\s+/)
        .filter(Boolean)
        .forEach((name) => classes.add(name));
    },
  });

  Object.defineProperty(element, "innerHTML", {
    get() {
      return this._innerHTML ?? "";
    },
    set(value) {
      this._innerHTML = value;
      this.children = [];
    },
  });

  return element;
}

async function loadPanelScript() {
  const elements = new Map(
    [
      "connectButton",
      "refreshButton",
      "disconnectButton",
      "saveClientJsonButton",
      "statusLine",
      "errorBanner",
      "clientJsonInput",
      "accountBadge",
      "accountName",
      "accountEmail",
      "agendaLabel",
      "agendaTitle",
      "agendaList",
      "tabUpcoming",
      "tabDaily",
      "tabWeekly",
      "taskLinkStatus",
      "accountToggleButton",
      "accountDetailsPanel",
      "settingsToggleButton",
      "calendarSettingsPanel",
      "calendarSettingsList",
      "calendarSettingsStatus",
      "saveCalendarSettingsButton",
    ].map((id) => [id, createElement(id.startsWith("tab") || id.endsWith("Button") ? "button" : "div")]),
  );

  globalThis.document = {
    getElementById(id) {
      return elements.get(id) ?? null;
    },
    createElement,
  };

  globalThis.window = {
    addEventListener() {},
    __TAURI__: {
      core: {
        async invoke(command) {
          if (command === "plugin_query_data") {
            return JSON.stringify({
              status: {
                connected: true,
                clientConfigured: true,
                clientJsonUploaded: true,
                effectiveClientId: "client-id",
                connectedAccount: {
                  name: "Richard Hao",
                  email: "test@example.com",
                },
                lastSyncAt: "2026-03-19T12:00:00Z",
                lastError: null,
              },
              upcoming: [],
              today: [],
              week: [
                {
                  id: "evt-1",
                  title: "apple one family payment - @Wind_Ace",
                  startAt: "2026-03-20",
                  endAt: "2026-03-20",
                  allDay: true,
                  calendarName: "Primary",
                  htmlLink: "https://calendar.google.com/event?eid=abc",
                  meetingUrl: "https://meet.google.com/abc-defg-hij",
                  location: null,
                },
              ],
              calendars: [
                {
                  id: "primary",
                  name: "Primary",
                  primary: true,
                  enabled: true,
                  accessRole: "owner",
                },
                {
                  id: "team@example.com",
                  name: "Team",
                  primary: false,
                  enabled: false,
                  accessRole: "reader",
                },
              ],
              eventLinkStatuses: [
                {
                  eventId: "evt-1",
                  taskId: "task-1",
                  status: "linked",
                },
              ],
            });
          }
          return JSON.stringify({ ok: true });
        },
      },
    },
  };

  globalThis.setInterval = () => 1;
  globalThis.clearInterval = () => {};

  const script = readFileSync(
    resolve(import.meta.dir, "../../../plugins/google-calendar/ui/panel.js"),
    "utf8",
  );

  new Function(script)();
  await Promise.resolve();
  await Promise.resolve();

  return elements;
}

describe("google calendar panel runtime", () => {
  test("shows the event date for weekly all-day events", async () => {
    const elements = await loadPanelScript();

    elements.get("tabWeekly").click();

    const agendaList = elements.get("agendaList");
    expect(agendaList.children).toHaveLength(1);
    expect(agendaList.children[0].innerHTML).toContain("Mar");
    expect(agendaList.children[0].innerHTML).toContain("All day");
    expect(agendaList.children[0].innerHTML).toContain("Join meeting");
    expect(agendaList.children[0].innerHTML).toContain("Linked");
    expect(agendaList.children[0].innerHTML).not.toContain("Add to tasks");
    expect(agendaList.children[0].innerHTML).toContain("View linked task");
  });

  test("renders stored calendars in settings", async () => {
    const elements = await loadPanelScript();

    const settingsList = elements.get("calendarSettingsList");
    expect(settingsList.innerHTML).toContain("Primary");
    expect(settingsList.innerHTML).toContain("Team");
    expect(settingsList.innerHTML).toContain("Primary calendar");
  });

  test("keeps account and settings collapsed by default when connected", async () => {
    const elements = await loadPanelScript();

    expect(elements.get("accountDetailsPanel").classList.contains("hidden")).toBe(true);
    expect(elements.get("calendarSettingsPanel").classList.contains("hidden")).toBe(true);
    expect(elements.get("accountToggleButton").attributes.get("aria-expanded")).toBe("false");
    expect(elements.get("settingsToggleButton").attributes.get("aria-expanded")).toBe("false");
  });

  test("expands account and settings sections when toggled", async () => {
    const elements = await loadPanelScript();

    elements.get("accountToggleButton").click();
    elements.get("settingsToggleButton").click();

    expect(elements.get("accountDetailsPanel").classList.contains("hidden")).toBe(false);
    expect(elements.get("calendarSettingsPanel").classList.contains("hidden")).toBe(false);
    expect(elements.get("accountToggleButton").attributes.get("aria-expanded")).toBe("true");
    expect(elements.get("settingsToggleButton").attributes.get("aria-expanded")).toBe("true");
  });
});
