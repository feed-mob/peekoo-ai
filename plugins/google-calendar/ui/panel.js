const connectButton = document.getElementById("connectButton");
const refreshButton = document.getElementById("refreshButton");
const disconnectButton = document.getElementById("disconnectButton");
const saveClientJsonButton = document.getElementById("saveClientJsonButton");
const statusLine = document.getElementById("statusLine");
const errorBanner = document.getElementById("errorBanner");
const clientJsonInput = document.getElementById("clientJsonInput");
const accountCard = document.getElementById("accountCard");
const accountName = document.getElementById("accountName");
const accountEmail = document.getElementById("accountEmail");
const upcomingList = document.getElementById("upcomingList");
const todayList = document.getElementById("todayList");
const weekList = document.getElementById("weekList");

let oauthFlowId = null;
let pollHandle = null;

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
  if (event.allDay) {
    return "All day";
  }
  try {
    return new Date(event.startAt).toLocaleString([], {
      weekday: "short",
      hour: "numeric",
      minute: "2-digit",
    });
  } catch {
    return event.startAt;
  }
}

function renderList(root, events, emptyTitle) {
  root.innerHTML = "";
  if (!events.length) {
    const empty = document.createElement("article");
    empty.className = "empty-card";
    empty.innerHTML = `<strong>${emptyTitle}</strong><p class="hero-copy">Nothing scheduled here right now.</p>`;
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

function applySnapshot(snapshot) {
  const { status } = snapshot;
  const connectedAccount = status.connectedAccount;

  if (!status.clientConfigured) {
    statusLine.textContent = "Upload your Google OAuth client.json file before connecting.";
  } else if (!status.connected) {
    statusLine.textContent = "Connect your Google Calendar to load upcoming, daily, and weekly views.";
  } else if (status.lastSyncAt) {
    statusLine.textContent = `Last synced ${new Date(status.lastSyncAt).toLocaleString()}.`;
  } else {
    statusLine.textContent = "Connected. Pulling your agenda now.";
  }

  if (status.clientJsonUploaded && status.effectiveClientId) {
    statusLine.textContent += ` Client loaded: ${status.effectiveClientId}.`;
  }

  if (connectedAccount?.email) {
    accountCard.classList.remove("hidden");
    accountName.textContent = connectedAccount.name || "Google account";
    accountEmail.textContent = connectedAccount.email;
    disconnectButton.disabled = false;
  } else {
    accountCard.classList.add("hidden");
    accountName.textContent = "";
    accountEmail.textContent = "";
    disconnectButton.disabled = !status.connected;
  }

  connectButton.disabled = !status.clientConfigured;

  showError(status.lastError ?? null);
  renderList(upcomingList, snapshot.upcoming, "No upcoming events");
  renderList(todayList, snapshot.today, "No daily events");
  renderList(weekList, snapshot.week, "No weekly events");
}

async function refreshSnapshot(refresh = false) {
  try {
    const snapshot = await invoke("google_calendar_panel_snapshot", { refresh });
    applySnapshot(snapshot);
  } catch (error) {
    showError(String(error));
  }
}

async function pollOauthStatus() {
  if (!oauthFlowId) {
    return;
  }
  try {
    const result = await invoke("google_calendar_connect_status", {
      req: { flowId: oauthFlowId },
    });
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

connectButton.addEventListener("click", async () => {
  showError(null);
  try {
    const result = await invoke("google_calendar_connect_start");
    oauthFlowId = result.flowId;
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
    await invoke("google_calendar_disconnect");
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
    await invoke("google_calendar_set_client_json", { clientJson });
    clientJsonInput.value = "";
    await refreshSnapshot(false);
  } catch (error) {
    showError(String(error));
  }
});

void refreshSnapshot(false);
