const statusRoot = document.getElementById("status");
const summaryRoot = document.getElementById("summary");
const refreshButton = document.getElementById("refreshButton");

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
  const minutes = Math.ceil(seconds / 60);
  if (minutes < 60) {
    return `${minutes} min`;
  }

  const hours = Math.floor(minutes / 60);
  const remainder = minutes % 60;
  return remainder === 0 ? `${hours} hr` : `${hours} hr ${remainder} min`;
}

function renderStatus(status) {
  summaryRoot.innerHTML = "";
  statusRoot.innerHTML = "";

  const pill = document.createElement("div");
  pill.className = `pill ${status.pomodoro_active ? "active" : ""}`;
  pill.textContent = status.pomodoro_active
    ? "Pomodoro suppression active"
    : "Reminders running";
  summaryRoot.appendChild(pill);

  status.reminders.forEach((item) => {
    const card = document.createElement("article");
    card.className = "card";

    const titleRow = document.createElement("div");
    titleRow.className = "title-row";

    const title = document.createElement("h2");
    title.textContent = item.reminder_type.replaceAll("_", " ");

    const state = document.createElement("span");
    state.className = `state ${item.active ? "ready" : "paused"}`;
    state.textContent = item.active ? "Scheduled" : "Paused";

    titleRow.append(title, state);

    const meta = document.createElement("p");
    meta.className = "meta";
    meta.textContent = item.active
      ? `Next reminder in ${formatSeconds(item.time_remaining_secs)}`
      : "Waiting for reminders to resume";

    const interval = document.createElement("p");
    interval.className = "interval";
    interval.textContent = `Every ${item.interval_min} min`;

    const dismiss = document.createElement("button");
    dismiss.textContent = "Reset timer";
    dismiss.disabled = !item.active;
    dismiss.addEventListener("click", async () => {
      await callTool("health_dismiss", { reminder_type: item.reminder_type });
      await refresh();
    });

    card.append(titleRow, meta, interval, dismiss);
    statusRoot.appendChild(card);
  });
}

async function refresh() {
  const status = await callTool("health_get_status");
  if (status) {
    renderStatus(status);
  }
}

refreshButton.addEventListener("click", () => {
  void refresh();
});

void refresh();
