/**
 * Health Reminders Panel Logic - Mission Dashboard V7
 * Optimized Synchronization & Performance
 */

const REMINDER_KEYS = {
  water: { id: "Water", api_key: "water" },
  eye: { id: "Eye", api_key: "eye_rest" },
  stand: { id: "Stand", api_key: "standup" }
};

let currentStatusSnapshot = null;
let isUserDraggingSlider = false; 

function formatTime(seconds) {
  if (seconds <= 0) return "DUE";
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
}

async function refreshStatus() {
  if (isUserDraggingSlider) return; 

  try {
    const status = await window.__TAURI__.core.invoke("plugin_call_tool", {
      toolName: "health_get_status",
      argsJson: "{}",
    });
    
    currentStatusSnapshot = status;
    renderStatus(status);
    syncSettingsFromBackend(status.config);
  } catch (err) {
    console.error("Health Status Error:", err);
  }
}

function renderStatus(status) {
  if (!status) return;
  const { water, eye_rest, standup, config } = status;

  // Global Mute State
  const globalMuted = !config.global_enabled;
  document.getElementById("iconMuted").style.display = globalMuted ? "block" : "none";
  document.getElementById("iconActive").style.display = globalMuted ? "none" : "block";
  document.getElementById("btnMuteAll").style.borderColor = globalMuted ? "var(--color-danger)" : "";

  const items = [
    { key: 'water', data: water, enabled: config.water_enabled },
    { key: 'eye', data: eye_rest, enabled: config.eye_rest_enabled },
    { key: 'stand', data: standup, enabled: config.standup_enabled }
  ];

  items.forEach(item => {
    const suffix = item.key.charAt(0).toUpperCase() + item.key.slice(1);
    const card = document.getElementById(`card${suffix}`);
    const timeEl = document.getElementById(`time${suffix}`);
    const progressBar = document.getElementById(`pb${suffix}`);
    const stateEl = document.getElementById(`state${suffix}`);

    if (item.enabled && !globalMuted) {
      card.classList.remove("disabled");
      stateEl.textContent = "ACTIVE TRACKING";
      
      const intervalSec = item.data.interval_min * 60;
      const remainingSec = Math.max(0, item.data.time_remaining_secs);
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
    const suffix = REMINDER_KEYS[key].id;
    const toggle = document.getElementById(`toggle${suffix}`);
    const range = document.getElementById(`range${suffix}`);
    const valDisplay = document.getElementById(`val${suffix}`);
    
    // Smoothly update if value changed remotely (avoid jitter while dragging happens elsewhere)
    if (Math.abs(range.value - config[`${key}_interval_min`]) > 0) {
        range.value = config[`${key}_interval_min`];
        valDisplay.textContent = `${range.value}m`;
    }
    toggle.checked = config[`${key}_enabled`];
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
      await window.__TAURI__.core.invoke("plugin_call_tool", {
        toolName: "health_configure",
        argsJson: JSON.stringify(config),
      });
      isUserDraggingSlider = false;
      refreshStatus();
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
     await window.__TAURI__.core.invoke("plugin_call_tool", {
        toolName: "health_configure",
        argsJson: JSON.stringify({ global_enabled: targetState }),
      });
      refreshStatus();
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
      await window.__TAURI__.core.invoke("plugin_call_tool", {
        toolName: "health_dismiss",
        argsJson: JSON.stringify({ reminder_type: REMINDER_KEYS[key].api_key }),
      });
      refreshStatus();
    } catch (err) {}
  };
});

// Run
refreshStatus();
setInterval(refreshStatus, 1000);
