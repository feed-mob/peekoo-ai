const connectButton = document.getElementById("connectButton");
const refreshButton = document.getElementById("refreshButton");
const disconnectButton = document.getElementById("disconnectButton");
const saveClientJsonButton = document.getElementById("saveClientJsonButton");
const statusLine = document.getElementById("statusLine");
const errorBanner = document.getElementById("errorBanner");
const clientJsonInput = document.getElementById("clientJsonInput");
const accountBadge = document.getElementById("accountBadge");
const accountName = document.getElementById("accountName");
const accountEmail = document.getElementById("accountEmail");
const agendaLabel = document.getElementById("agendaLabel");
const agendaTitle = document.getElementById("agendaTitle");
const agendaList = document.getElementById("agendaList");
const tabUpcoming = document.getElementById("tabUpcoming");
const tabDaily = document.getElementById("tabDaily");
const tabWeekly = document.getElementById("tabWeekly");

const TAB_CONFIG = {
  upcoming: {
    button: tabUpcoming,
    label: "Upcoming",
    title: "Next 5 events",
    emptyTitle: "No upcoming events",
    key: "upcoming",
  },
  daily: {
    button: tabDaily,
    label: "Daily",
    title: "Today",
    emptyTitle: "No daily events",
    key: "today",
  },
  weekly: {
    button: tabWeekly,
    label: "Weekly",
    title: "This week",
    emptyTitle: "No weekly events",
    key: "week",
  },
};

let oauthFlowId = null;
let pollHandle = null;
let activeTab = "upcoming";
let lastSnapshot = null;

async function invoke(command, payload = {}) {
  return window.__TAURI__.core.invoke(command, payload);
}

function showError(message) {
  if (!message) {
    errorBanner.classList.add("hidden");
    errorBanner.textContent = "";
    return;
  }
  errorBanner.classList.remove("hidden");
  errorBanner.textContent = message;
}

function formatWhen(event) {
  try {
    const date = new Date(event.startAt);
    if (event.allDay) {
      return `${date.toLocaleDateString([], {
        weekday: "short",
        month: "short",
        day: "numeric",
      })} • All day`;
    }
    return date.toLocaleString([], {
      weekday: "short",
      month: "short",
      day: "numeric",
      hour: "numeric",
      minute: "2-digit",
    });
  } catch {
    return event.allDay ? `${event.startAt} • All day` : event.startAt;
  }
}

function renderList(root, events, emptyTitle) {
  root.innerHTML = "";
  if (!events.length) {
    const empty = document.createElement("article");
    empty.className = "empty-card";
    empty.innerHTML = `<strong>${emptyTitle}</strong><p class="empty-copy">Nothing scheduled here right now.</p>`;
    root.appendChild(empty);
    return;
  }

  events.forEach((event) => {
    const card = document.createElement("article");
    card.className = "event-card";
    const location = event.location
      ? `<p class="event-location">${event.location}</p>`
      : "";
    card.innerHTML = `
      <p class="event-time">${formatWhen(event)}</p>
      <h3 class="event-title">${event.title}</h3>
      ${location}
      <p class="meta-line">${event.calendarName}</p>
    `;
    root.appendChild(card);
  });
}

function renderAgenda(snapshot) {
  const tab = TAB_CONFIG[activeTab];
  const events = snapshot[tab.key] || [];

  agendaLabel.textContent = tab.label;
  agendaTitle.textContent = tab.title;
  renderList(agendaList, events, tab.emptyTitle);

  Object.entries(TAB_CONFIG).forEach(([key, config]) => {
    const isActive = key === activeTab;
    config.button.classList.toggle("active", isActive);
    config.button.setAttribute("aria-selected", isActive ? "true" : "false");
  });
}

function applySnapshot(snapshot) {
  lastSnapshot = snapshot;
  const { status } = snapshot;
  const connectedAccount = status.connectedAccount;

  if (!status.clientConfigured) {
    statusLine.textContent = "Upload your Google OAuth client.json file before connecting.";
  } else if (!status.connected) {
    statusLine.textContent = "Connect Google Calendar to load your agenda views.";
  } else if (status.lastSyncAt) {
    statusLine.textContent = `Connected. Last synced ${new Date(status.lastSyncAt).toLocaleString()}.`;
  } else {
    statusLine.textContent = "Connected. Pulling your agenda now.";
  }

  if (connectedAccount?.email) {
    accountName.textContent = connectedAccount.name || "Google account";
    accountEmail.textContent = connectedAccount.email;
    accountBadge.textContent = "Connected";
    accountBadge.classList.add("connected");
  } else {
    accountName.textContent = status.clientConfigured ? "Not connected" : "Client setup required";
    accountEmail.textContent = status.clientConfigured
      ? "Connect Google Calendar to start syncing events."
      : "Upload your Google OAuth client.json to enable connection.";
    accountBadge.textContent = "Offline";
    accountBadge.classList.remove("connected");
  }

  disconnectButton.disabled = !status.connected;
  connectButton.disabled = !status.clientConfigured;

  showError(status.lastError ?? null);
  renderAgenda(snapshot);
}

async function refreshSnapshot(refresh = false) {
  try {
    if (refresh) {
      await invoke("plugin_call_panel_tool", {
        pluginKey: "google-calendar",
        toolName: "google_calendar_refresh",
        argsJson: "{}",
      });
    }
    const raw = await invoke("plugin_query_data", {
      pluginKey: "google-calendar",
      providerName: "panel_snapshot",
    });
    applySnapshot(JSON.parse(raw));
  } catch (error) {
    showError(String(error));
  }
}

async function pollOauthStatus() {
  if (!oauthFlowId) {
    return;
  }
  try {
    const raw = await invoke("plugin_call_panel_tool", {
      pluginKey: "google-calendar",
      toolName: "google_calendar_connect_status",
      argsJson: JSON.stringify({ flow_id: oauthFlowId }),
    });
    const result = JSON.parse(raw);
    if (result.status === "completed") {
      oauthFlowId = null;
      stopOauthPolling();
      await refreshSnapshot(true);
      return;
    }
    if (result.status === "failed" || result.status === "expired") {
      oauthFlowId = null;
      stopOauthPolling();
      showError(result.error ?? "Google Calendar connection failed.");
    }
  } catch (error) {
    stopOauthPolling();
    showError(String(error));
  }
}

function startOauthPolling() {
  stopOauthPolling();
  pollHandle = setInterval(() => {
    void pollOauthStatus();
  }, 1500);
}

function stopOauthPolling() {
  if (pollHandle !== null) {
    clearInterval(pollHandle);
    pollHandle = null;
  }
}

function setActiveTab(tab) {
  activeTab = tab;
  if (lastSnapshot) {
    renderAgenda(lastSnapshot);
  }
}

tabUpcoming.addEventListener("click", () => setActiveTab("upcoming"));
tabDaily.addEventListener("click", () => setActiveTab("daily"));
tabWeekly.addEventListener("click", () => setActiveTab("weekly"));

connectButton.addEventListener("click", async () => {
  showError(null);
  try {
    const raw = await invoke("plugin_call_panel_tool", {
      pluginKey: "google-calendar",
      toolName: "google_calendar_connect_start",
      argsJson: "{}",
    });
    const result = JSON.parse(raw);
    oauthFlowId = result.flowId;
    await invoke("system_open_url", { url: result.authorizeUrl });
    startOauthPolling();
  } catch (error) {
    showError(String(error));
  }
});

refreshButton.addEventListener("click", () => {
  void refreshSnapshot(true);
});

disconnectButton.addEventListener("click", async () => {
  showError(null);
  try {
    await invoke("plugin_call_panel_tool", {
      pluginKey: "google-calendar",
      toolName: "google_calendar_disconnect",
      argsJson: "{}",
    });
    await refreshSnapshot(false);
  } catch (error) {
    showError(String(error));
  }
});

saveClientJsonButton.addEventListener("click", async () => {
  showError(null);
  try {
    const file = clientJsonInput.files?.[0];
    if (!file) {
      showError("Choose your Google OAuth client.json file first.");
      return;
    }
    const clientJson = await file.text();
    await invoke("plugin_call_panel_tool", {
      pluginKey: "google-calendar",
      toolName: "google_calendar_set_client_json",
      argsJson: JSON.stringify({ clientJson }),
    });
    clientJsonInput.value = "";
    await refreshSnapshot(false);
  } catch (error) {
    showError(String(error));
  }
});

void refreshSnapshot(false);
