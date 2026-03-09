const statusRoot = document.getElementById("status");
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

function renderStatus(status) {
  statusRoot.innerHTML = "";

  status.reminders.forEach((item) => {
    const card = document.createElement("article");
    card.className = "card";

    const title = document.createElement("h2");
    title.textContent = item.reminder_type.replaceAll("_", " ");

    const meta = document.createElement("p");
    meta.className = "meta";
    meta.textContent = `${item.minutes_since_last}/${item.interval_min} min`;

    const progress = document.createElement("div");
    progress.className = "progress";
    const bar = document.createElement("div");
    bar.className = `bar ${item.is_due ? "due" : ""}`;
    bar.style.width = `${Math.min(100, (item.minutes_since_last / item.interval_min) * 100)}%`;
    progress.appendChild(bar);

    const dismiss = document.createElement("button");
    dismiss.textContent = item.is_due ? "Dismiss" : "Reset";
    dismiss.addEventListener("click", async () => {
      await callTool("health_dismiss", { reminder_type: item.reminder_type });
      await refresh();
    });

    card.append(title, meta, progress, dismiss);
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
