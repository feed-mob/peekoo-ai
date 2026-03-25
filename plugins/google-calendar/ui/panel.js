const connectButton = document.getElementById("connectButton");
const refreshButton = document.getElementById("refreshButton");
const disconnectButton = document.getElementById("disconnectButton");
const saveClientJsonButton = document.getElementById("saveClientJsonButton");
const statusLine = document.getElementById("statusLine");
const errorBanner = document.getElementById("errorBanner");
const successBanner = document.getElementById("successBanner");
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
const taskLinkStatus = document.getElementById("taskLinkStatus");
const accountToggleButton = document.getElementById("accountToggleButton");
const accountDetailsPanel = document.getElementById("accountDetailsPanel");
const settingsToggleButton = document.getElementById("settingsToggleButton");
const calendarSettingsPanel = document.getElementById("calendarSettingsPanel");
const calendarSettingsList = document.getElementById("calendarSettingsList");
const calendarSettingsStatus = document.getElementById("calendarSettingsStatus");
const saveCalendarSettingsButton = document.getElementById("saveCalendarSettingsButton");
const taskModal = document.getElementById("taskModal");
const taskModalEyebrow = document.getElementById("taskModalEyebrow");
const taskModalTitle = document.getElementById("taskModalTitle");
const taskModalSubtitle = document.getElementById("taskModalSubtitle");
const taskSearchInput = document.getElementById("taskSearchInput");
const taskModalList = document.getElementById("taskModalList");
const taskModalUnlink = document.getElementById("taskModalUnlink");
const taskModalCancel = document.getElementById("taskModalCancel");
const taskModalConfirm = document.getElementById("taskModalConfirm");

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
const eventStatuses = new Map();
let availableTasks = [];
let modalEvent = null;
let modalSelectedTaskId = "";
let modalMode = "link";
let successBannerTimer = null;
let accountOpen = null;
let settingsOpen = false;
let calendarSelections = [];

function setTaskLinkStatus(message) {
  if (taskLinkStatus) {
    taskLinkStatus.textContent = message;
  }
}

function showSuccess(message) {
  if (!successBanner) {
    return;
  }
  if (successBannerTimer) {
    clearTimeout(successBannerTimer);
    successBannerTimer = null;
  }
  if (!message) {
    successBanner.classList.add("hidden");
    successBanner.textContent = "";
    return;
  }
  successBanner.classList.remove("hidden");
  successBanner.textContent = message;
  successBannerTimer = setTimeout(() => {
    successBanner.classList.add("hidden");
    successBanner.textContent = "";
    successBannerTimer = null;
  }, 3500);
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

async function loadTasks() {
  try {
    const tasks = await invoke("list_tasks");
    availableTasks = Array.isArray(tasks)
      ? tasks.filter(
          (task) =>
            task &&
            typeof task.id === "string" &&
            typeof task.title === "string",
        )
      : [];
    renderTaskModalList();
  } catch (error) {
    availableTasks = [];
    renderTaskModalList();
    showError(String(error));
  }
}

function filteredTasks() {
  const keyword = (taskSearchInput?.value ?? "").trim().toLowerCase();
  const pendingTasks = availableTasks.filter(
    (task) => task.status === "todo" || task.status === "in_progress",
  );
  if (!keyword) {
    return pendingTasks;
  }
  return pendingTasks.filter((task) => {
    const haystack = [task.title, ...(Array.isArray(task.labels) ? task.labels : [])]
      .join(" ")
      .toLowerCase();
    return haystack.includes(keyword);
  });
}

function renderTaskModalList() {
  if (!taskModalList) {
    return;
  }

  taskModalList.innerHTML = "";
  const tasks = modalMode === "view" ? [] : filteredTasks();

  if (modalMode === "view") {
    const linkedTask = findLinkedTask();
    const detail = document.createElement("div");
    detail.className = "task-modal__detail";
    if (!linkedTask) {
      detail.innerHTML = `<p class="task-modal__empty">Linked task not found. It may have been deleted.</p>`;
    } else {
      detail.innerHTML = `
        <p class="task-modal__detail-row"><strong>${escapeHtml(linkedTask.title)}</strong></p>
        <p class="task-modal__detail-row">Status: ${escapeHtml(linkedTask.status.replaceAll("_", " "))}</p>
        ${linkedTask.scheduled_start_at ? `<p class="task-modal__detail-row">Start: ${escapeHtml(linkedTask.scheduled_start_at)}</p>` : ""}
        ${linkedTask.scheduled_end_at ? `<p class="task-modal__detail-row">End: ${escapeHtml(linkedTask.scheduled_end_at)}</p>` : ""}
        ${linkedTask.description ? `<p class="task-modal__detail-row">${escapeHtml(linkedTask.description)}</p>` : ""}
      `;
    }
    taskModalList.appendChild(detail);
    taskModalUnlink?.classList.remove("hidden");
    if (taskModalConfirm) {
      taskModalConfirm.disabled = false;
      taskModalConfirm.textContent = "Close";
    }
    if (taskModalCancel) {
      taskModalCancel.textContent = "Done";
    }
    return;
  }

  taskModalUnlink?.classList.add("hidden");

  if (!tasks.length) {
    const empty = document.createElement("p");
    empty.className = "task-modal__empty";
    empty.textContent = "No matching pending tasks.";
    taskModalList.appendChild(empty);
  } else {
    tasks.forEach((task) => {
      const item = document.createElement("button");
      item.type = "button";
      item.className = `task-modal__item${modalSelectedTaskId === task.id ? " is-selected" : ""}`;
      item.innerHTML = `${escapeHtml(task.title)}<span class="task-modal__meta">${escapeHtml(task.status.replaceAll("_", " "))}</span>`;
      item.addEventListener("click", () => {
        modalSelectedTaskId = task.id;
        renderTaskModalList();
      });
      taskModalList.appendChild(item);
    });
  }

  if (taskModalConfirm) {
    taskModalConfirm.disabled = !modalSelectedTaskId;
    taskModalConfirm.textContent = "Link task";
  }
  if (taskModalCancel) {
    taskModalCancel.textContent = "Cancel";
  }
}

function findLinkedTask() {
  if (!modalEvent) {
    return null;
  }
  const linkInfo = eventStatuses.get(modalEvent.id);
  if (!linkInfo?.taskId) {
    return null;
  }
  return availableTasks.find((task) => task.id === linkInfo.taskId) ?? null;
}

function closeTaskModal() {
  modalEvent = null;
  modalSelectedTaskId = "";
  modalMode = "link";
  if (taskSearchInput) {
    taskSearchInput.value = "";
    taskSearchInput.classList.remove("hidden");
  }
  taskModal?.classList.add("hidden");
}

async function openTaskModal(event) {
  modalMode = "link";
  modalEvent = event;
  modalSelectedTaskId = "";
  if (taskModalEyebrow) {
    taskModalEyebrow.textContent = "Link Event";
  }
  if (taskModalTitle) {
    taskModalTitle.textContent = event.title;
  }
  if (taskModalSubtitle) {
    taskModalSubtitle.textContent = "Choose a todo or in-progress task to link this calendar event.";
  }
  taskModal?.classList.remove("hidden");
  await loadTasks();
  taskSearchInput?.focus?.();
}

async function openLinkedTaskModal(event) {
  modalMode = "view";
  modalEvent = event;
  if (taskModalEyebrow) {
    taskModalEyebrow.textContent = "Linked Task";
  }
  if (taskModalTitle) {
    taskModalTitle.textContent = event.title;
  }
  if (taskModalSubtitle) {
    taskModalSubtitle.textContent = "This calendar event is already associated with a task.";
  }
  if (taskSearchInput) {
    taskSearchInput.value = "";
    taskSearchInput.classList.add("hidden");
  }
  taskModal?.classList.remove("hidden");
  await loadTasks();
}

async function submitTaskLink() {
  if (modalMode === "view") {
    closeTaskModal();
    return;
  }
  if (!modalEvent || !modalSelectedTaskId) {
    return;
  }
  try {
    await invoke("plugin_call_tool", {
      toolName: "google_calendar_link_existing_event_to_task",
      argsJson: JSON.stringify({
        taskId: modalSelectedTaskId,
        eventId: modalEvent.id,
        linkType: "linked",
      }),
    });
    const selectedTask = availableTasks.find((task) => task.id === modalSelectedTaskId);
    eventStatuses.set(modalEvent.id, { status: "linked", taskId: modalSelectedTaskId });
    const message = `Linked event to "${selectedTask?.title ?? "selected task"}".`;
    setTaskLinkStatus(message);
    showSuccess(message);
    closeTaskModal();
    await refreshSnapshot(false);
  } catch (error) {
    showError(String(error));
  }
}

async function unlinkTaskFromEvent() {
  if (!modalEvent) {
    return;
  }
  const linkInfo = eventStatuses.get(modalEvent.id);
  if (!linkInfo?.taskId) {
    closeTaskModal();
    return;
  }

  try {
    await invoke("plugin_call_tool", {
      toolName: "google_calendar_unlink_task_event",
      argsJson: JSON.stringify({
        taskId: linkInfo.taskId,
        eventId: modalEvent.id,
      }),
    });
    eventStatuses.delete(modalEvent.id);
    const message = `Unlinked task from "${modalEvent.title}".`;
    setTaskLinkStatus(message);
    showSuccess(message);
    closeTaskModal();
    await refreshSnapshot(false);
  } catch (error) {
    showError(String(error));
  }
}

async function createTaskFromEvent(event) {
  const schedule = deriveTaskSchedule(event);
  const description = buildTaskDescription(event);

  try {
    const createdTask = await invoke("create_task", {
      title: event.title,
      priority: "medium",
      assignee: "user",
      labels: [],
      description,
      scheduled_start_at: schedule.startAt,
      scheduled_end_at: schedule.endAt,
      estimated_duration_min: null,
      recurrence_rule: null,
      recurrence_time_of_day: null,
    });
    const syncedTask = await ensureTaskSchedule(createdTask, schedule, description);
    await invoke("plugin_call_tool", {
      toolName: "google_calendar_link_existing_event_to_task",
      argsJson: JSON.stringify({
        taskId: syncedTask.id,
        eventId: event.id,
        linkType: "created",
      }),
    });
    eventStatuses.set(event.id, { status: "created", taskId: syncedTask.id });
    const message = `Created task \"${syncedTask.title}\" and linked event \"${event.title}\".`;
    setTaskLinkStatus(message);
    showSuccess(message);
    await refreshSnapshot(false);
  } catch (error) {
    showError(String(error));
  }
}

async function ensureTaskSchedule(task, schedule, description) {
  const needsStartSync = (schedule.startAt ?? null) !== (task.scheduled_start_at ?? null);
  const needsEndSync = (schedule.endAt ?? null) !== (task.scheduled_end_at ?? null);
  const needsDescriptionSync = (description ?? null) !== (task.description ?? null);

  if (!needsStartSync && !needsEndSync && !needsDescriptionSync) {
    return task;
  }

  return invoke("update_task", {
    id: task.id,
    scheduled_start_at: schedule.startAt,
    scheduled_end_at: schedule.endAt,
    description,
  });
}

function buildTaskDescription(event) {
  const sections = [];
  if (event.description) {
    sections.push(event.description.trim());
  }
  const details = [];
  if (event.location) {
    details.push(`Location: ${event.location}`);
  }
  if (event.htmlLink) {
    details.push(`Calendar event: ${event.htmlLink}`);
  }
  if (event.meetingUrl) {
    details.push(`Meeting URL: ${event.meetingUrl}`);
  }
  if (details.length) {
    sections.push(details.join("\n"));
  }
  return sections.length ? sections.join("\n\n") : null;
}

async function openExternalUrl(url) {
  if (!url) {
    return;
  }
  try {
    await invoke("system_open_url", { url });
  } catch (error) {
    showError(String(error));
  }
}

function deriveTaskSchedule(event) {
  if (event.allDay && typeof event.startAt === "string") {
    const day = event.startAt.slice(0, 10);
    return {
      startAt: `${day}T00:00:00Z`,
      endAt: `${day}T23:59:00Z`,
    };
  }
  return {
    startAt: event.startAt ?? null,
    endAt: event.endAt ?? null,
  };
}

function renderEventStatusBadge(eventId) {
  const linkInfo = eventStatuses.get(eventId);
  if (!linkInfo?.status) {
    return "";
  }
  const label = linkInfo.status === "created" ? "Created" : "Linked";
  const tone = linkInfo.status === "created" ? "status-created" : "status-linked";
  return `<span class="event-status ${tone}">${label}</span>`;
}

function renderTaskActionGroup(event) {
  if (eventStatuses.has(event.id)) {
    return `<button type="button" class="event-action event-view-task" data-event-id="${escapeHtml(event.id)}">View linked task</button>`;
  }

  return `
    <div class="event-action-group">
      <button type="button" class="event-action is-primary event-add-task" data-event-id="${escapeHtml(event.id)}">Add to tasks</button>
      <div class="event-action-menu" data-event-menu="${escapeHtml(event.id)}">
        <button type="button" class="event-action event-action-option event-create-task" data-event-id="${escapeHtml(event.id)}">Create new task</button>
        <button type="button" class="event-action event-action-option event-link-task" data-event-id="${escapeHtml(event.id)}">Link existing task</button>
      </div>
    </div>
  `;
}

async function invoke(command, payload = {}) {
  return window.__TAURI__.core.invoke(command, payload);
}

function showError(message) {
  showSuccess(null);
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
    const joinAction = event.meetingUrl
      ? `<button type="button" class="event-action event-join-meeting" data-url="${escapeHtml(event.meetingUrl)}">Join meeting</button>`
      : "";
    const openAction = event.htmlLink
      ? `<button type="button" class="event-action event-open-link" data-url="${escapeHtml(event.htmlLink)}">Open event</button>`
      : "";
    card.innerHTML = `
      <p class="event-time">${formatWhen(event)}</p>
      <div class="event-title-row">
        <h3 class="event-title">${escapeHtml(event.title)}</h3>
        ${renderEventStatusBadge(event.id)}
      </div>
      ${location}
      <p class="meta-line">${event.calendarName}</p>
      <div class="event-actions">
        ${renderTaskActionGroup(event)}
        ${openAction}
        ${joinAction}
      </div>
    `;

    if (typeof card.querySelector === "function") {
      const addTaskButton = card.querySelector(".event-add-task");
      const actionMenu = card.querySelector(".event-action-menu");
      if (addTaskButton) {
        addTaskButton.addEventListener("click", () => {
          actionMenu?.classList.toggle("is-open");
        });
      }

      const createButton = card.querySelector(".event-create-task");
      if (createButton) {
        createButton.addEventListener("click", () => {
          actionMenu?.classList.remove("is-open");
          void createTaskFromEvent(event);
        });
      }

      const linkButton = card.querySelector(".event-link-task");
      if (linkButton) {
        linkButton.addEventListener("click", async () => {
          actionMenu?.classList.remove("is-open");
          await openTaskModal(event);
        });
      }

      const viewTaskButton = card.querySelector(".event-view-task");
      if (viewTaskButton) {
        viewTaskButton.addEventListener("click", async () => {
          await openLinkedTaskModal(event);
        });
      }

      const joinButton = card.querySelector(".event-join-meeting");
      if (joinButton) {
        joinButton.addEventListener("click", async () => {
          await openExternalUrl(event.meetingUrl);
        });
      }

      const openButton = card.querySelector(".event-open-link");
      if (openButton) {
        openButton.addEventListener("click", async () => {
          await openExternalUrl(event.htmlLink);
        });
      }
    }

    root.appendChild(card);
  });
}

function renderAccountSection(status) {
  if (!accountDetailsPanel) {
    return;
  }

  if (accountOpen === null) {
    accountOpen = !status.connected;
  }

  if (!status.connected) {
    accountOpen = true;
  }

  accountDetailsPanel.classList.toggle("hidden", !accountOpen);
  accountToggleButton?.setAttribute("aria-expanded", accountOpen ? "true" : "false");
}

function renderCalendarSettings() {
  if (!calendarSettingsPanel || !calendarSettingsList || !calendarSettingsStatus) {
    return;
  }

  calendarSettingsPanel.classList.toggle("hidden", !settingsOpen);
  settingsToggleButton?.setAttribute("aria-expanded", settingsOpen ? "true" : "false");

  if (!calendarSelections.length) {
    calendarSettingsStatus.textContent = "Connect and refresh to load available calendars.";
    calendarSettingsList.innerHTML = '<p class="settings-empty">No calendar choices available yet.</p>';
    if (saveCalendarSettingsButton) {
      saveCalendarSettingsButton.disabled = true;
    }
    return;
  }

  calendarSettingsStatus.textContent = "Enabled calendars will be included in future syncs.";
  calendarSettingsList.innerHTML = calendarSelections
    .map(
      (calendar) => `
        <label class="settings-item">
          <div class="settings-item__copy">
            <div class="settings-item__title-row">
              <p class="settings-item__title">${escapeHtml(calendar.name)}</p>
              ${calendar.primary ? '<span class="settings-badge">Primary calendar</span>' : ""}
            </div>
            <p class="settings-item__meta">${escapeHtml(calendar.accessRole ?? "reader")}</p>
          </div>
          <input class="settings-checkbox" type="checkbox" data-calendar-id="${escapeHtml(calendar.id)}" ${calendar.enabled ? "checked" : ""} />
        </label>
      `,
    )
    .join("");

  if (typeof calendarSettingsList.querySelectorAll === "function") {
    calendarSettingsList.querySelectorAll("[data-calendar-id]").forEach((input) => {
      input.addEventListener("change", (event) => {
        const calendarId = event.target?.getAttribute?.("data-calendar-id");
        const enabled = Boolean(event.target?.checked);
        calendarSelections = calendarSelections.map((calendar) =>
          calendar.id === calendarId ? { ...calendar, enabled } : calendar,
        );
      });
    });
  }

  if (saveCalendarSettingsButton) {
    saveCalendarSettingsButton.disabled = false;
  }
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
  if (Array.isArray(snapshot.eventLinkStatuses)) {
    eventStatuses.clear();
    snapshot.eventLinkStatuses.forEach((entry) => {
      if (
        entry &&
        typeof entry.eventId === "string" &&
        typeof entry.status === "string" &&
        typeof entry.taskId === "string"
      ) {
        eventStatuses.set(entry.eventId, {
          status: entry.status,
          taskId: entry.taskId,
        });
      }
    });
  }
  const { status } = snapshot;
  const connectedAccount = status.connectedAccount;
  calendarSelections = Array.isArray(snapshot.calendars) ? snapshot.calendars.map((calendar) => ({ ...calendar })) : [];

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
  if (saveCalendarSettingsButton) {
    saveCalendarSettingsButton.disabled = !status.connected || !calendarSelections.length;
  }

  renderAccountSection(status);
  showError(status.lastError ?? null);
  renderCalendarSettings();
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
taskSearchInput?.addEventListener("input", () => {
  renderTaskModalList();
});
accountToggleButton?.addEventListener("click", () => {
  accountOpen = !accountOpen;
  if (accountDetailsPanel) {
    accountDetailsPanel.classList.toggle("hidden", !accountOpen);
  }
  accountToggleButton?.setAttribute("aria-expanded", accountOpen ? "true" : "false");
});
settingsToggleButton?.addEventListener("click", () => {
  settingsOpen = !settingsOpen;
  renderCalendarSettings();
});
saveCalendarSettingsButton?.addEventListener("click", async () => {
  showError(null);
  try {
    const raw = await invoke("plugin_call_panel_tool", {
      pluginKey: "google-calendar",
      toolName: "google_calendar_update_calendar_selection",
      argsJson: JSON.stringify({
        calendars: calendarSelections.map((calendar) => ({
          id: calendar.id,
          enabled: Boolean(calendar.enabled),
        })),
      }),
    });
    applySnapshot(JSON.parse(raw));
    showSuccess("Saved calendar selection.");
  } catch (error) {
    showError(String(error));
  }
});
taskModalCancel?.addEventListener("click", () => {
  closeTaskModal();
});
taskModalUnlink?.addEventListener("click", () => {
  void unlinkTaskFromEvent();
});
taskModalConfirm?.addEventListener("click", () => {
  void submitTaskLink();
});
taskModal?.addEventListener("click", (event) => {
  if (event.target === taskModal || event.target?.classList?.contains("task-modal__backdrop")) {
    closeTaskModal();
  }
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
