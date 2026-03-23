const statusRoot = document.getElementById("status");
const summaryRoot = document.getElementById("summary");
const refreshButton = document.getElementById("refreshButton");
const POLL_INTERVAL_MS = 30000;
const COUNTDOWN_TICK_MS = 1000;

let pollIntervalId = null;
let countdownIntervalId = null;
let lastStatusSnapshot = null;
let lastStatusSnapshotAt = 0;
let refreshPromise = null;

async function callTool(toolName, args = {}) {
  try {
    const result = await window.__TAURI__.core.invoke("plugin_call_tool", {
      toolName,
      argsJson: JSON.stringify(args),
    });
    return JSON.parse(result);
  } catch (err) {
    console.error(`callTool(${toolName}) failed:`, err);
    return null;
  }
}

function formatSeconds(seconds) {
  if (seconds <= 0) {
    return "now";
  }

  const minutes = Math.max(1, Math.floor(seconds / 60));
  if (minutes <= 1) {
    return "~1 min";
  }

  if (minutes < 60) {
    return `~${minutes} min`;
  }

  const hours = Math.floor(minutes / 60);
  const remainder = minutes % 60;
  return remainder === 0 ? `~${hours} hr` : `~${hours} hr ${remainder} min`;
}

function nextDueText(seconds) {
  return seconds <= 0 ? "Due now" : `Next in ${formatSeconds(seconds)}`;
}

function summaryContent(status) {
  if (!status || !status.reminders) return { dotClass: "status-dot quiet", title: "Error", subtitle: "Could not load health status" };
  const nextReminder = status.reminders
    .filter((item) => item.active)
    .sort((left, right) => left.time_remaining_secs - right.time_remaining_secs)[0];

  if (!nextReminder) {
    return {
      dotClass: "status-dot quiet",
      title: "Reminders paused",
      subtitle: "Restart a timer to resume wellness nudges",
    };
  }

  return {
    dotClass: "status-dot",
    title: "Reminders running",
    subtitle:
      nextReminder.time_remaining_secs <= 0
        ? `${labelFor(nextReminder.reminder_type)} due now`
        : `${labelFor(nextReminder.reminder_type)} next in ${formatSeconds(nextReminder.time_remaining_secs)}`,
  };
}

function labelFor(reminderType) {
  return reminderType
    .replaceAll("_", " ")
    .replace(/\b\w/g, (character) => character.toUpperCase());
}

function renderStatus(status) {
  if (!status || !status.reminders) {
    console.warn("renderStatus: missing status or reminders", status);
    return;
  }
  summaryRoot.innerHTML = "";
  statusRoot.innerHTML = "";

  const summary = summaryContent(status);
  const badge = document.createElement("div");
  badge.className = "status-badge";

  const dot = document.createElement("span");
  dot.className = summary.dotClass;

  const textContainer = document.createElement("div");
  textContainer.className = "status-text";

  const title = document.createElement("strong");
  title.textContent = summary.title;

  const subtitle = document.createElement("span");
  subtitle.textContent = summary.subtitle;

  textContainer.append(title, subtitle);
  badge.append(dot, textContainer);
  summaryRoot.appendChild(badge);

  // Define each reminder type so we can iterate stably (or use status.reminders)
  const types = ["water", "eye_rest", "standup"];
  
  types.forEach(type => {
    const item = status.reminders.find(r => r.reminder_type === type);
    if (!item) return;

    const isEnabled = status.config[`${type}_enabled`];
    const intervalValue = status.config[`${type}_interval_min`];

    const card = document.createElement("article");
    card.className = "reminder-card";
    if (!isEnabled) card.style.opacity = "0.7";

    const header = document.createElement("div");
    header.className = "card-header";

    const titleContainer = document.createElement("div");
    titleContainer.className = "card-title";

    const title = document.createElement("h3");
    title.textContent = labelFor(item.reminder_type);

    const badge = document.createElement("span");
    badge.className = `card-badge ${item.active ? "active" : "paused"}`;
    badge.textContent = item.active ? "Active" : "Paused";

    titleContainer.append(title, badge);

    // Toggle Switch (the "sliding button")
    const toggleLabel = document.createElement("label");
    toggleLabel.className = "switch";
    const toggleInput = document.createElement("input");
    toggleInput.type = "checkbox";
    toggleInput.checked = isEnabled;
    toggleInput.addEventListener("change", (e) => {
      const payload = {};
      payload[`${type}_enabled`] = e.target.checked;
      void callTool("health_configure", payload).then(refresh);
    });
    const slider = document.createElement("span");
    slider.className = "slider";
    toggleLabel.append(toggleInput, slider);

    header.append(titleContainer, toggleLabel);

    const body = document.createElement("div");
    body.className = "card-body";

    // Duration controls
    const controls = document.createElement("div");
    controls.className = "card-controls";

    const durationLabel = document.createElement("div");
    durationLabel.className = "duration-control";
    durationLabel.textContent = "Every";
    
    const input = document.createElement("input");
    input.type = "number";
    input.className = "input-number";
    input.value = intervalValue;
    input.min = type === "standup" ? 10 : 5;
    input.max = 180;
    
    let debounceTimer;
    input.addEventListener("input", (e) => {
      clearTimeout(debounceTimer);
      debounceTimer = setTimeout(() => {
        const value = parseInt(e.target.value);
        if (!isNaN(value)) {
          const payload = {};
          payload[`${type}_interval_min`] = value;
          void callTool("health_configure", payload).then(refresh);
        }
      }, 800);
    });

    const minsText = document.createElement("span");
    minsText.textContent = "min";
    minsText.style.fontSize = "12px";
    minsText.style.color = "var(--text-secondary)";

    durationLabel.append(input, minsText);
    controls.appendChild(durationLabel);

    const nextDue = document.createElement("p");
    nextDue.className = "card-next-due";
    nextDue.style.fontSize = "12px";
    nextDue.textContent = item.active
      ? nextDueText(item.time_remaining_secs)
      : "Next: --";
    
    controls.appendChild(nextDue);
    body.append(controls);

    const footer = document.createElement("div");
    footer.className = "card-footer";

    const restartBtn = document.createElement("button");
    restartBtn.className = "btn-restart";
    restartBtn.textContent = "Restart Timer";
    restartBtn.disabled = !isEnabled;
    restartBtn.style.width = "100%";
    restartBtn.addEventListener("click", async () => {
      await callTool("health_dismiss", { reminder_type: item.reminder_type });
      await refresh();
    });

    footer.appendChild(restartBtn);

    card.append(header, body, footer);
    statusRoot.appendChild(card);
  });

  // Linked Reminders (Automation)
  if (status.event_reminders && status.event_reminders.length > 0) {
    const title = document.createElement("h4");
    title.className = "section-title";
    title.textContent = "Linked Reminders";
    statusRoot.appendChild(title);

    status.event_reminders.forEach((item, index) => {
      const card = document.createElement("article");
      card.className = "reminder-card automation";
      
      const header = document.createElement("div");
      header.className = "card-header";

      const titleContainer = document.createElement("div");
      titleContainer.className = "card-title";

      const h3 = document.createElement("h3");
      h3.textContent = item.message;

      const badge = document.createElement("span");
      badge.className = "card-badge linked";
      badge.textContent = `When ${item.event_name.split(':')[0]} complete`;

      titleContainer.append(h3, badge);
      header.append(titleContainer);

      const body = document.createElement("div");
      body.className = "card-body";
      body.style.fontSize = "12px";
      body.style.color = "var(--text-secondary)";
      body.textContent = `Created at ${new Date(item.created_at * 1000).toLocaleTimeString()}`;

      card.append(header, body);
      statusRoot.appendChild(card);
    });
  }
}

function countdownStatus(status, elapsedSeconds) {
  return {
    ...status,
    reminders: status.reminders.map((item) => ({
      ...item,
      time_remaining_secs: item.active
        ? Math.max(0, item.time_remaining_secs - elapsedSeconds)
        : item.time_remaining_secs,
    })),
  };
}

function renderLiveStatus() {
  if (!lastStatusSnapshot) {
    return;
  }

  const elapsedSeconds = Math.floor((Date.now() - lastStatusSnapshotAt) / 1000);
  renderStatus(countdownStatus(lastStatusSnapshot, elapsedSeconds));
}

function hasDueReminder() {
  if (!lastStatusSnapshot) {
    return false;
  }

  const elapsedSeconds = Math.floor((Date.now() - lastStatusSnapshotAt) / 1000);
  return lastStatusSnapshot.reminders.some(
    (item) => item.active && item.time_remaining_secs - elapsedSeconds <= 0,
  );
}

async function refresh() {
  if (refreshPromise) {
    return refreshPromise;
  }

  summaryRoot.innerHTML = '<div class="status-badge"><span class="status-dot quiet"></span><div class="status-text"><strong>Loading...</strong></div></div>';

  refreshPromise = callTool("health_get_status")
    .then(async (status) => {
      if (status) {
        lastStatusSnapshot = status;
        lastStatusSnapshotAt = Date.now();
        renderLiveStatus();
        startPolling();
        startCountdown();
      } else {
        // If it failed, maybe the plugin is not enabled/loaded?
        // Try to enable it once and retry.
        console.log("health_get_status returned null, attempting to enable plugin...");
        try {
          await window.__TAURI__.core.invoke("plugin_enable", { pluginKey: "health-reminders" });
          // Retry once
          const secondTry = await callTool("health_get_status");
          if (secondTry) {
            lastStatusSnapshot = secondTry;
            lastStatusSnapshotAt = Date.now();
            renderLiveStatus();
            startPolling();
            startCountdown();
            return;
          }
        } catch (e) {
          console.error("Failed to force-enable plugin:", e);
        }
        
        summaryRoot.innerHTML = '<div class="status-badge"><span class="status-dot active"></span><div class="status-text"><strong>Error</strong><span>Failed to load health status. Is the plugin active?</span></div></div>';
      }
    })
    .catch(err => {
      summaryRoot.innerHTML = `<div class="status-badge"><span class="status-dot active"></span><div class="status-text"><strong>Error</strong><span>${err.message || 'Unknown error'}</span></div></div>`;
    })
    .finally(() => {
      refreshPromise = null;
    });

  return refreshPromise;
}

function startPolling() {
  if (pollIntervalId !== null) {
    clearInterval(pollIntervalId);
  }

  pollIntervalId = setInterval(() => {
    void refresh();
  }, POLL_INTERVAL_MS);
}

function startCountdown() {
  if (countdownIntervalId !== null) {
    clearInterval(countdownIntervalId);
  }

  countdownIntervalId = setInterval(() => {
    if (hasDueReminder()) {
      return refresh();
    }

    renderLiveStatus();
  }, COUNTDOWN_TICK_MS);
}

refreshButton.addEventListener("click", () => {
  void refresh();
});

void refresh();
