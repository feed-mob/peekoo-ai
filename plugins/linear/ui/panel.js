const invoke = window.__TAURI__.core.invoke;

const statusBadge = document.getElementById("statusBadge");
const statusLine = document.getElementById("statusLine");
const workspaceLine = document.getElementById("workspaceLine");
const userLine = document.getElementById("userLine");
const apiKeyInput = document.getElementById("apiKeyInput");
const apiKeyHint = document.getElementById("apiKeyHint");
const disconnectButton = document.getElementById("disconnectButton");
const syncLinearButton = document.getElementById("syncLinearButton");
const syncLocalButton = document.getElementById("syncLocalButton");
const syncTargetSelect = document.getElementById("syncTargetSelect");
const syncStateList = document.getElementById("syncStateList");
const autoPushToggle = document.getElementById("autoPushToggle");
const lastSyncLine = document.getElementById("lastSyncLine");
const successBanner = document.getElementById("successBanner");
const errorBanner = document.getElementById("errorBanner");
const MASKED_KEY_VALUE = "****************";
const CURRENT_ASSIGNEE_SENTINEL = "__current__";
const DEFAULT_SYNC_STATES = ["backlog", "todo", "in review", "in progress"];
const SYNC_STATE_OPTIONS = [
  { value: "triage", label: "Triage" },
  { value: "backlog", label: "Backlog" },
  { value: "todo", label: "Todo" },
  { value: "in progress", label: "In Progress" },
  { value: "in review", label: "In Review" },
  { value: "done", label: "Done" },
  { value: "canceled", label: "Canceled" },
];
let isConnected = false;
let apiKeySaveTimer = null;
let settingsSaveTimer = null;

function showError(message) {
  if (!message) {
    errorBanner.classList.add("hidden");
    errorBanner.textContent = "";
    return;
  }
  errorBanner.textContent = message;
  errorBanner.classList.remove("hidden");
}

function showSuccess(message) {
  if (!message) {
    successBanner.classList.add("hidden");
    successBanner.textContent = "";
    return;
  }
  successBanner.textContent = message;
  successBanner.classList.remove("hidden");
  setTimeout(() => {
    successBanner.classList.add("hidden");
    successBanner.textContent = "";
  }, 3000);
}

function formatStatus(status) {
  switch (status) {
    case "connected":
      return "Connected";
    case "syncing":
      return "Syncing";
    case "error":
      return "Error";
    case "disconnected":
      return "Disconnected";
    default:
      return "Unknown";
  }
}

function renderSyncTargets(status, teams, preferences) {
  syncTargetSelect.innerHTML = "";
  const me = document.createElement("option");
  me.value = `assignee:${CURRENT_ASSIGNEE_SENTINEL}`;
  me.textContent = "Current account (Me)";
  syncTargetSelect.appendChild(me);

  teams.forEach((team) => {
    const option = document.createElement("option");
    option.value = `team:${team.id}`;
    option.textContent = `Team: ${team.name} (${team.key})`;
    syncTargetSelect.appendChild(option);
  });

  const selected = preferences.assigneeId
    ? `assignee:${preferences.assigneeId}`
    : preferences.defaultTeamId
      ? `team:${preferences.defaultTeamId}`
      : `assignee:${CURRENT_ASSIGNEE_SENTINEL}`;

  if (Array.from(syncTargetSelect.options).some((option) => option.value === selected)) {
    syncTargetSelect.value = selected;
  } else {
    syncTargetSelect.value = `assignee:${CURRENT_ASSIGNEE_SENTINEL}`;
  }
}

function normalizeStateName(value) {
  return (value || "").trim().toLowerCase().replaceAll("_", " ").replaceAll("-", " ");
}

function renderSyncStateList(preferences) {
  syncStateList.innerHTML = "";
  const selected = new Set(
    (preferences.syncStateNames && preferences.syncStateNames.length > 0
      ? preferences.syncStateNames
      : DEFAULT_SYNC_STATES).map(normalizeStateName),
  );

  SYNC_STATE_OPTIONS.forEach((option) => {
    const label = document.createElement("label");
    label.className = "sync-state-item";

    const input = document.createElement("input");
    input.type = "checkbox";
    input.value = option.value;
    input.checked = selected.has(normalizeStateName(option.value));
    input.addEventListener("change", scheduleSettingsSave);

    const text = document.createElement("span");
    text.textContent = option.label;

    label.appendChild(input);
    label.appendChild(text);
    syncStateList.appendChild(label);
  });
}

function applySnapshot(snapshot) {
  const { status, teams, preferences, mappingCount } = snapshot;
  const pretty = formatStatus(status.status);
  isConnected = Boolean(status.connected);

  statusBadge.textContent = pretty;
  statusLine.textContent = `Status: ${pretty} · ${status.connected ? "Connected" : "Not connected"}`;
  workspaceLine.textContent = status.workspaceName ? `Workspace: ${status.workspaceName}` : "";
  userLine.textContent = status.userEmail
    ? `User: ${status.userName || ""} ${status.userEmail}`.trim()
    : "";

  autoPushToggle.checked = Boolean(preferences.autoPushNewTasks);
  renderSyncTargets(status, teams || [], preferences);
  renderSyncStateList(preferences);

  lastSyncLine.textContent = status.lastSyncAt
    ? `Last sync: ${status.lastSyncAt} · Linked tasks: ${mappingCount}`
    : "No sync yet";

  disconnectButton.disabled = !status.connected;
  syncLinearButton.disabled = !status.connected;
  syncLocalButton.disabled = !status.connected;

  if (isConnected) {
    apiKeyHint.textContent = "API key is already saved securely. You only need to enter it again when replacing it.";
    if (!apiKeyInput.value || apiKeyInput.value === MASKED_KEY_VALUE) {
      apiKeyInput.value = MASKED_KEY_VALUE;
    }
  } else {
    apiKeyHint.textContent = "API key will be stored securely and auto-saved.";
    apiKeyInput.value = "";
    apiKeyInput.placeholder = "lin_api_...";
  }

  showError(status.status === "error" ? (status.lastError || "Sync failed.") : null);
}

async function refreshSnapshot(runSync = false) {
  try {
    if (runSync) {
      await invoke("plugin_call_panel_tool", {
        pluginKey: "linear",
        toolName: "linear_sync_now",
        argsJson: "{}",
      });
    }

    const raw = await invoke("plugin_query_data", {
      pluginKey: "linear",
      providerName: "panel_snapshot",
    });
    applySnapshot(JSON.parse(raw));
    return true;
  } catch (error) {
    showError(String(error));
    return false;
  }
}

async function runManualSync(toolName, button, syncingLabel, doneLabel) {
  const previousLabel = button.textContent;
  const wasDisabled = button.disabled;
  button.disabled = true;
  button.textContent = syncingLabel;
  try {
    await invoke("plugin_call_panel_tool", {
      pluginKey: "linear",
      toolName,
      argsJson: "{}",
    });
    await refreshSnapshot(false);
    showSuccess(doneLabel);
  } catch (error) {
    showError(String(error));
  } finally {
    button.textContent = previousLabel;
    if (!wasDisabled) {
      button.disabled = false;
    }
  }
}

async function saveApiKeyIfNeeded() {
  try {
    showError(null);
    const nextApiKey = (apiKeyInput.value || "").trim();
    if (!nextApiKey || (isConnected && nextApiKey === MASKED_KEY_VALUE)) {
      return;
    }
    await invoke("plugin_call_panel_tool", {
      pluginKey: "linear",
      toolName: "linear_set_api_key",
      argsJson: JSON.stringify({
        apiKey: nextApiKey,
      }),
    });
    showSuccess("API key auto-saved.");
    await refreshSnapshot(false);
  } catch (error) {
    showError(String(error));
  }
}

async function saveSyncSettings() {
  try {
    const selectedStates = Array.from(
      syncStateList.querySelectorAll("input[type='checkbox']:checked"),
    ).map((entry) => normalizeStateName(entry.value));

    await invoke("plugin_call_panel_tool", {
      pluginKey: "linear",
      toolName: "linear_set_sync_settings",
      argsJson: JSON.stringify({
        assigneeId: syncTargetSelect.value.startsWith("assignee:")
          ? syncTargetSelect.value.replace("assignee:", "")
          : null,
        defaultTeamId: syncTargetSelect.value.startsWith("team:")
          ? syncTargetSelect.value.replace("team:", "")
          : null,
        syncStateNames: selectedStates.length > 0 ? selectedStates : DEFAULT_SYNC_STATES,
        autoPushNewTasks: Boolean(autoPushToggle.checked),
      }),
    });
    showSuccess("Settings auto-saved.");
  } catch (error) {
    showError(String(error));
  }
}

apiKeyInput.addEventListener("focus", () => {
  if (apiKeyInput.value === MASKED_KEY_VALUE) {
    apiKeyInput.value = "";
  }
});

apiKeyInput.addEventListener("input", () => {
  if (apiKeyInput.value === MASKED_KEY_VALUE) {
    return;
  }
  if (apiKeySaveTimer) {
    window.clearTimeout(apiKeySaveTimer);
  }
  apiKeySaveTimer = window.setTimeout(() => {
    void saveApiKeyIfNeeded();
  }, 700);
});

apiKeyInput.addEventListener("blur", () => {
  if (apiKeySaveTimer) {
    window.clearTimeout(apiKeySaveTimer);
    apiKeySaveTimer = null;
  }
  void saveApiKeyIfNeeded();
});

disconnectButton.addEventListener("click", async () => {
  try {
    await invoke("plugin_call_panel_tool", {
      pluginKey: "linear",
      toolName: "linear_disconnect",
      argsJson: "{}",
    });
    showSuccess("Disconnected.");
    await refreshSnapshot(false);
  } catch (error) {
    showError(String(error));
  }
});

syncLinearButton.addEventListener("click", () =>
  runManualSync("linear_sync_linear", syncLinearButton, "Syncing...", "Linear pull completed."),
);

syncLocalButton.addEventListener("click", () =>
  runManualSync("linear_sync_local", syncLocalButton, "Syncing...", "Local push completed."),
);

function scheduleSettingsSave() {
  if (settingsSaveTimer) {
    window.clearTimeout(settingsSaveTimer);
  }
  settingsSaveTimer = window.setTimeout(() => {
    void saveSyncSettings();
  }, 300);
}

syncTargetSelect.addEventListener("change", scheduleSettingsSave);
autoPushToggle.addEventListener("change", scheduleSettingsSave);

void refreshSnapshot(false);
