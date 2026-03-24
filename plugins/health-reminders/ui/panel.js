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

  status.reminders.forEach((item) => {
    const card = document.createElement("article");
    card.className = "reminder-card";

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
    header.appendChild(titleContainer);

    const body = document.createElement("div");
    body.className = "card-body";

    const interval = document.createElement("p");
    interval.className = "card-interval";
    interval.textContent = `Every ${item.interval_min} min`;

    const nextDue = document.createElement("p");
    nextDue.className = "card-next-due";
    nextDue.textContent = item.active
      ? nextDueText(item.time_remaining_secs)
      : "Waiting for reminders to resume";

    const meta = document.createElement("p");
    meta.className = "card-meta";
    meta.textContent = item.active
      ? "Quiet reminder will appear automatically when due"
      : "This reminder will stay quiet until scheduling resumes";

    body.append(interval, nextDue, meta);

    const footer = document.createElement("div");
    footer.className = "card-footer";

    const restartBtn = document.createElement("button");
    restartBtn.className = "btn-restart";
    restartBtn.textContent = "Restart timer";
    restartBtn.disabled = !item.active;
    restartBtn.addEventListener("click", async () => {
      await callTool("health_dismiss", { reminder_type: item.reminder_type });
      await refresh();
    });

    footer.appendChild(restartBtn);

    card.append(header, body, footer);
    statusRoot.appendChild(card);
  });
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

  refreshPromise = callTool("health_get_status")
    .then((status) => {
      if (status) {
        lastStatusSnapshot = status;
        lastStatusSnapshotAt = Date.now();
        renderLiveStatus();
        startPolling();
        startCountdown();
      }
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
