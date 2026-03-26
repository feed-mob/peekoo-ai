const refreshButton = document.getElementById("refreshButton");
const disconnectButton = document.getElementById("disconnectButton");
const connectButton = document.getElementById("connectButton");
const toggleSettingsBtn = document.getElementById("toggleSettingsBtn");
const statusLine = document.getElementById("statusLine");
const statusIndicator = document.getElementById("statusIndicator");
const errorBanner = document.getElementById("errorBanner");
const clientJsonInput = document.getElementById("clientJsonInput");
const accountBadge = document.getElementById("accountBadge");
const accountName = document.getElementById("accountName");
const accountEmail = document.getElementById("accountEmail");
const agendaTitle = document.getElementById("agendaTitle");
const agendaList = document.getElementById("agendaList");
const tabUpcoming = document.getElementById("tabUpcoming");
const tabDaily = document.getElementById("tabDaily");
const tabWeekly = document.getElementById("tabWeekly");
const settingsTray = document.getElementById("settingsTray");
const setupSection = document.getElementById("setupSection");

const TAB_CONFIG = {
  upcoming: {
    button: tabUpcoming,
    title: "Upcoming Agenda",
    emptyTitle: "No upcoming events",
    key: "upcoming",
  },
  daily: {
    button: tabDaily,
    title: "Today's Schedule",
    emptyTitle: "Focus time: No daily events",
    key: "today",
  },
  weekly: {
    button: tabWeekly,
    title: "This Week's Outlook",
    emptyTitle: "A clear week ahead",
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

function showSuccess(message) {
  // Temporarily show success in the status line
  const originalText = statusLine.textContent;
  const originalColor = statusIndicator.querySelector('.live-dot').style.background;
  
  statusLine.textContent = message;
  statusIndicator.querySelector('.live-dot').style.background = 'var(--success)';
  
  setTimeout(() => {
    statusLine.textContent = originalText;
    statusIndicator.querySelector('.live-dot').style.background = originalColor;
  }, 3000);
}

function formatTime(dateStr) {
  const date = new Date(dateStr);
  return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

function formatDay(dateStr) {
  const date = new Date(dateStr);
  return date.toLocaleDateString([], { month: 'short', day: 'numeric' });
}

function renderList(root, events, emptyTitle) {
  root.innerHTML = "";
  if (!events || !events.length) {
    const empty = document.createElement("div");
    empty.className = "empty-state";
    empty.innerHTML = `
      <div class="empty-icon">🗓️</div>
      <p style="font-weight: 600; color: var(--text-primary); margin-bottom: 4px;">${emptyTitle}</p>
      <p style="font-size: 13px;">Nothing scheduled here right now.</p>
    `;
    root.appendChild(empty);
    return;
  }

  events.forEach((event, index) => {
    const card = document.createElement("article");
    card.className = "event-card";
    card.style.animationDelay = `${index * 0.05}s`;
    
    const startTime = formatTime(event.startAt);
    const endTime = event.endAt ? formatTime(event.endAt) : "";
    const dayMark = formatDay(event.startAt);

    card.innerHTML = `
      <div class="event-accent"></div>
      <div class="event-time-block">
        <span class="time-start">${event.allDay ? "All Day" : startTime}</span>
        <span class="time-end">${event.allDay ? dayMark : endTime}</span>
      </div>
      <div class="event-info">
        <h3 class="event-title">${event.title}</h3>
        <div class="event-meta">
          <span class="meta-pill">${event.calendarName}</span>
          ${event.location ? `<span class="location-snippet">📍 ${event.location}</span>` : ""}
        </div>
      </div>
    `;
    root.appendChild(card);
  });
}

function applySnapshot(snapshot) {
  console.log("[Google Calendar] Applying snapshot:", snapshot);
  lastSnapshot = snapshot;
  const { status } = snapshot;
  const connectedAccount = status.connectedAccount;

  // Header Status
  if (!status.clientConfigured) {
    statusLine.textContent = "Configuration required";
    statusIndicator.querySelector('.live-dot').style.background = 'var(--text-muted)';
  } else if (!status.connected) {
    statusLine.textContent = "Offline";
    statusIndicator.querySelector('.live-dot').style.background = 'var(--text-muted)';
  } else {
    statusLine.textContent = "Live";
    statusIndicator.querySelector('.live-dot').style.background = 'var(--success)';
  }

  // Account Tray
  if (connectedAccount?.email) {
    accountName.textContent = connectedAccount.name || "Google User";
    accountEmail.textContent = connectedAccount.email;
    accountBadge.textContent = "Connected";
    accountBadge.className = "status-pill online";
    setupSection.classList.add("hidden");
  } else {
    accountName.textContent = status.clientConfigured ? "Not Connected" : "Client Needed";
    accountEmail.textContent = status.clientConfigured 
      ? "Ready to authenticate." 
      : "Provide client.json to continue.";
    accountBadge.textContent = status.clientConfigured ? "Ready" : "Offline";
    accountBadge.className = "status-pill";
    setupSection.classList.remove("hidden");
  }

  disconnectButton.style.display = status.connected ? "block" : "none";
  connectButton.disabled = !status.clientConfigured;
  connectButton.textContent = status.connected ? "Re-authenticate" : "Authorize Now";

  showError(status.lastError ?? null);
  
  // Render List
  const tab = TAB_CONFIG[activeTab];
  agendaTitle.textContent = tab.title;
  renderList(agendaList, snapshot[tab.key] || [], tab.emptyTitle);

  // Update tabs
  Object.entries(TAB_CONFIG).forEach(([key, config]) => {
    config.button.classList.toggle("active", key === activeTab);
  });
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
  if (!oauthFlowId) return;
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
      settingsTray.classList.remove("visible");
      await refreshSnapshot(true);
      return;
    }
    if (result.status === "failed" || result.status === "expired") {
      oauthFlowId = null;
      stopOauthPolling();
      showError(result.error ?? "Authentication failed.");
    }
  } catch (error) {
    stopOauthPolling();
    showError(String(error));
  }
}

function startOauthPolling() {
  stopOauthPolling();
  pollHandle = setInterval(() => pollOauthStatus(), 2000);
}

function stopOauthPolling() {
  if (pollHandle) {
    clearInterval(pollHandle);
    pollHandle = null;
  }
}

// Event Listeners
toggleSettingsBtn.addEventListener("click", () => {
  settingsTray.classList.toggle("visible");
});

tabUpcoming.addEventListener("click", () => {
  activeTab = "upcoming";
  applySnapshot(lastSnapshot);
});
tabDaily.addEventListener("click", () => {
  activeTab = "daily";
  applySnapshot(lastSnapshot);
});
tabWeekly.addEventListener("click", () => {
  activeTab = "weekly";
  applySnapshot(lastSnapshot);
});

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

refreshButton.addEventListener("click", () => refreshSnapshot(true));

disconnectButton.addEventListener("click", async () => {
  if (!confirm("Disconnect Google Calendar? Your local cache will be cleared.")) return;
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

clientJsonInput.addEventListener("change", async () => {
  const file = clientJsonInput.files?.[0];
  console.log("[Google Calendar] File selected:", file?.name);
  if (!file) return;
  showError(null);
  try {
    const clientJson = await file.text();
    console.log("[Google Calendar] File content length:", clientJson.length);
    console.log("[Google Calendar] Calling set_client_json tool...");
    const result = await invoke("plugin_call_panel_tool", {
      pluginKey: "google-calendar",
      toolName: "google_calendar_set_client_json",
      argsJson: JSON.stringify({ clientJson }),
    });
    console.log("[Google Calendar] Tool result:", result);
    clientJsonInput.value = "";
    console.log("[Google Calendar] Refreshing snapshot...");
    await refreshSnapshot(false);
    console.log("[Google Calendar] Upload complete");
    
    // Show success feedback
    showSuccess("Client configuration uploaded successfully!");
  } catch (error) {
    console.error("[Google Calendar] Upload error:", error);
    showError(String(error));
  }
});

void refreshSnapshot(false);
