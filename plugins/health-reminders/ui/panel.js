/**
 * Health Reminders Panel Logic - Mission Dashboard V7
 * Optimized Synchronization & Performance
 */

const REMINDER_KEYS = {
  water: {
    id: "Water",
    api_key: "water",
    config_key: "water_interval_min",
    enabled_key: "water_enabled"
  },
  eye: {
    id: "Eye",
    api_key: "eye_rest",
    config_key: "eye_rest_interval_min",
    enabled_key: "eye_rest_enabled"
  },
  stand: {
    id: "Stand",
    api_key: "standup",
    config_key: "standup_interval_min",
    enabled_key: "standup_enabled"
  }
};

let currentStatusSnapshot = null;
let isUserDraggingSlider = false; 

function formatTime(seconds) {
  if (seconds <= 0) return "DUE";
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
}

async function callHealthTool(toolName, args = {}) {
  const result = await window.__TAURI__.core.invoke("plugin_call_tool", {
    toolName,
    argsJson: JSON.stringify(args),
  });

  return typeof result === "string" ? JSON.parse(result) : result;
}

function normalizeReminderStatus(status) {
  return Object.fromEntries((status?.reminders || []).map(reminder => [reminder.reminder_type, reminder]));
}

async function refreshStatus() {
  if (isUserDraggingSlider) return; 

  try {
    const status = await callHealthTool("health_get_status");
    
    currentStatusSnapshot = status;
    renderStatus(status);
    syncSettingsFromBackend(status.config);
  } catch (err) {
    console.error("Health Status Error:", err);
  }
}

function renderStatus(status) {
  if (!status || !status.config) return;

  const { config } = status;
  const reminders = normalizeReminderStatus(status);

  // Global Mute State
  const globalMuted = !config.global_enabled;
  document.getElementById("iconMuted").style.display = globalMuted ? "block" : "none";
  document.getElementById("iconActive").style.display = globalMuted ? "none" : "block";
  document.getElementById("btnMuteAll").style.borderColor = globalMuted ? "var(--color-danger)" : "";

  const items = Object.entries(REMINDER_KEYS).map(([key, entry]) => ({
    key,
    reminder: reminders[entry.api_key],
    enabled: Boolean(config[entry.enabled_key]),
    intervalMin: Number(config[entry.config_key] || 0),
  }));

  items.forEach(item => {
    const suffix = item.key.charAt(0).toUpperCase() + item.key.slice(1);
    const card = document.getElementById(`card${suffix}`);
    const timeEl = document.getElementById(`time${suffix}`);
    const progressBar = document.getElementById(`pb${suffix}`);
    const stateEl = document.getElementById(`state${suffix}`);

    if (item.enabled && !globalMuted) {
      card.classList.remove("disabled");
      stateEl.textContent = "ACTIVE TRACKING";
      
      const intervalSec = item.intervalMin * 60;
      const remainingSec = item.reminder
        ? Math.max(0, item.reminder.time_remaining_secs)
        : intervalSec;
      const progress = intervalSec > 0 ? (1 - (remainingSec / intervalSec)) : 0;
      
      timeEl.textContent = formatTime(remainingSec);
      timeEl.classList.toggle("imminent", remainingSec <= 0);
      card.classList.toggle("due", remainingSec <= 0);
      progressBar.style.width = `${Math.min(100, Math.round(progress * 100))}%`;
    } else {
      card.classList.add("disabled");
      card.classList.remove("due");
      stateEl.textContent = globalMuted ? "SYSTEM MUTED" : "DISABLED";
      timeEl.textContent = "OFF";
      timeEl.classList.remove("imminent");
      progressBar.style.width = `0%`;
    }
  });
}

function syncSettingsFromBackend(config) {
  if (!config || isUserDraggingSlider) return;
  
  Object.keys(REMINDER_KEYS).forEach(key => {
    const entry = REMINDER_KEYS[key];
    const suffix = entry.id;
    const toggle = document.getElementById(`toggle${suffix}`);
    const range = document.getElementById(`range${suffix}`);
    const valDisplay = document.getElementById(`val${suffix}`);
    const intervalValue = config[entry.config_key];
     
    // Smoothly update if value changed remotely (avoid jitter while dragging happens elsewhere)
    if (String(range.value) !== String(intervalValue)) {
        range.value = intervalValue;
        valDisplay.textContent = `${range.value}m`;
    }
    toggle.checked = Boolean(config[entry.enabled_key]);
  });
}

let syncDebounceTimer = null;
async function pushConfigUpdateToBackend() {
  const config = {
    water_interval_min: parseInt(document.getElementById("rangeWater").value),
    water_enabled: document.getElementById("toggleWater").checked,
    eye_rest_interval_min: parseInt(document.getElementById("rangeEye").value),
    eye_rest_enabled: document.getElementById("toggleEye").checked,
    standup_interval_min: parseInt(document.getElementById("rangeStand").value),
    standup_enabled: document.getElementById("toggleStand").checked,
    global_enabled: currentStatusSnapshot ? currentStatusSnapshot.config.global_enabled : true
  };

  if (syncDebounceTimer) clearTimeout(syncDebounceTimer);
  syncDebounceTimer = setTimeout(async () => {
    try {
      const status = await callHealthTool("health_configure", config);
      currentStatusSnapshot = status;
      isUserDraggingSlider = false;
      renderStatus(status);
      syncSettingsFromBackend(status.config);
    } catch (err) {
      console.error("Sync Failed:", err);
      isUserDraggingSlider = false;
    }
  }, 400);
}

function handleLocalInputFeedback(key) {
  isUserDraggingSlider = true;
  const suffix = REMINDER_KEYS[key].id;
  const range = document.getElementById(`range${suffix}`);
  const valDisplay = document.getElementById(`val${suffix}`);
  const timeEl = document.getElementById(`time${suffix}`);

  valDisplay.textContent = `${range.value}m`;
  timeEl.textContent = `${range.value.toString().padStart(2, "0")}:00`;
  timeEl.style.color = "var(--text-muted)";
}

// Global Actions
document.getElementById("btnMuteAll").onclick = async () => {
  if (!currentStatusSnapshot) return;
  const targetState = !currentStatusSnapshot.config.global_enabled;
  try {
      const status = await callHealthTool("health_configure", { global_enabled: targetState });
      currentStatusSnapshot = status;
      renderStatus(status);
      syncSettingsFromBackend(status.config);
   } catch(e) {}
};

// Item Listeners
Object.keys(REMINDER_KEYS).forEach(key => {
  const suffix = REMINDER_KEYS[key].id;
  const range = document.getElementById(`range${suffix}`);
  const toggle = document.getElementById(`toggle${suffix}`);
  const resetBtn = document.getElementById(`reset${suffix}`);

  range.oninput = () => handleLocalInputFeedback(key);
  range.onchange = () => {
    document.getElementById(`time${suffix}`).style.color = "var(--text-primary)";
    pushConfigUpdateToBackend();
  };
  
  toggle.onchange = () => {
    isUserDraggingSlider = true; 
    pushConfigUpdateToBackend();
  };

  resetBtn.onclick = async () => {
    try {
      const status = await callHealthTool("health_dismiss", {
        reminder_type: REMINDER_KEYS[key].api_key,
      });
      currentStatusSnapshot = status;
      renderStatus(status);
      syncSettingsFromBackend(status.config);
    } catch (err) {}
  };
});

// Run
refreshStatus();
setInterval(refreshStatus, 1000);
