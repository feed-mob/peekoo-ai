#![cfg_attr(not(test), no_main)]

use chrono::{DateTime, Datelike, Days, Duration, NaiveDate, SecondsFormat, Utc, Weekday};
use peekoo_plugin_sdk::prelude::*;

const CLIENT_ID_KEY: &str = "client-id";
const CLIENT_SECRET_KEY: &str = "client-secret";
const TOKEN_BUNDLE_KEY: &str = "token-bundle";
const CONNECTED_ACCOUNT_KEY: &str = "connected-account";
const STATE_KEY: &str = "calendar-state";
const SYNC_SCHEDULE_KEY: &str = "calendar-sync";
const REMINDER_SCHEDULE_PREFIX: &str = "reminder:";
const GOOGLE_PROVIDER_ID: &str = "google-calendar";
const GOOGLE_AUTHORIZE_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_REDIRECT_URI: &str = "http://localhost:1455/auth/callback";
const GOOGLE_SCOPES: &str = "https://www.googleapis.com/auth/calendar openid email profile";
const DEFAULT_REFRESH_INTERVAL_SECS: i64 = 300;

const DEFAULT_UPCOMING_LIMIT: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct GoogleClientCredentials {
    client_id: String,
    client_secret: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoogleAccountProfile {
    email: String,
    name: Option<String>,
    picture: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenBundle {
    access_token: String,
    refresh_token: Option<String>,
    expires_at_epoch: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredCalendarState {
    last_sync_at: Option<String>,
    last_error: Option<String>,
    cached_events: Vec<CalendarEvent>,
    notified_event_ids: Vec<String>,
    #[serde(default)]
    task_links: Vec<TaskCalendarLink>,
    #[serde(default)]
    calendars: Vec<StoredGoogleCalendar>,
    oauth_flow_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredGoogleCalendar {
    id: String,
    name: String,
    primary: bool,
    access_role: String,
    enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskCalendarLink {
    task_id: String,
    event_id: String,
    #[serde(default = "default_link_type")]
    link_type: String,
}

fn default_link_type() -> String {
    "linked".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoogleCalendarStatusDto {
    connected: bool,
    client_configured: bool,
    client_json_uploaded: bool,
    effective_client_id: String,
    connected_account: Option<GoogleAccountProfile>,
    last_sync_at: Option<String>,
    last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoogleCalendarPanelDto {
    status: GoogleCalendarStatusDto,
    calendars: Vec<StoredGoogleCalendar>,
    upcoming: Vec<CalendarEventBucket>,
    today: Vec<CalendarEventBucket>,
    week: Vec<CalendarEventBucket>,
    event_link_statuses: Vec<EventLinkStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EventLinkStatus {
    event_id: String,
    task_id: String,
    status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoogleCalendarAgentEventsDto {
    connected: bool,
    last_sync_at: Option<String>,
    count: usize,
    events: Vec<CalendarEventDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoogleCalendarRefreshDto {
    connected: bool,
    last_sync_at: Option<String>,
    upcoming_count: usize,
    daily_count: usize,
    weekly_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoogleCalendarOauthStatusDto {
    status: String,
    connected: bool,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConnectStartDto {
    flow_id: String,
    authorize_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FlowIdInput {
    #[serde(alias = "flow_id")]
    flow_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CalendarEvent {
    id: String,
    title: String,
    start_at: String,
    end_at: String,
    all_day: bool,
    location: Option<String>,
    description: Option<String>,
    #[serde(default)]
    calendar_id: String,
    calendar_name: String,
    html_link: Option<String>,
    #[serde(default)]
    meeting_url: Option<String>,
    status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CalendarEventDto {
    id: String,
    title: String,
    start_at: String,
    end_at: String,
    all_day: bool,
    location: Option<String>,
    description: Option<String>,
    calendar_name: String,
    html_link: Option<String>,
    #[serde(default)]
    meeting_url: Option<String>,
    status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CalendarEventBucket {
    id: String,
    title: String,
    start_at: String,
    end_at: String,
    all_day: bool,
    location: Option<String>,
    description: Option<String>,
    calendar_name: String,
    html_link: Option<String>,
    #[serde(default)]
    meeting_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskCalendarEventActionResult {
    ok: bool,
    event: CalendarEventDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskCalendarEventListResult {
    task_id: String,
    count: usize,
    events: Vec<CalendarEventDto>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateTaskEventInput {
    task_id: String,
    title: String,
    start_at: String,
    end_at: String,
    description: Option<String>,
    location: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LinkTaskEventInput {
    task_id: String,
    event_id: String,
    link_type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskIdInput {
    task_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CalendarSelectionInput {
    id: String,
    enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateCalendarSelectionInput {
    calendars: Vec<CalendarSelectionInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BucketedCalendarEvents {
    upcoming: Vec<CalendarEventBucket>,
    today: Vec<CalendarEventBucket>,
    week: Vec<CalendarEventBucket>,
}

#[derive(Debug, Deserialize)]
struct GoogleClientJson {
    installed: Option<GoogleInstalledClient>,
    web: Option<GoogleInstalledClient>,
}

#[derive(Debug, Deserialize)]
struct GoogleInstalledClient {
    client_id: String,
    client_secret: String,
}

#[derive(Debug, Deserialize)]
struct GoogleAccountProfilePayload {
    email: Option<String>,
    name: Option<String>,
    picture: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GoogleEventsResponse {
    items: Vec<GoogleEventItem>,
}

#[derive(Debug, Deserialize)]
struct GoogleCalendarListResponse {
    items: Vec<GoogleCalendarListEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoogleCalendarListEntry {
    id: String,
    summary: Option<String>,
    summary_override: Option<String>,
    primary: Option<bool>,
    access_role: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GoogleEventItem {
    id: String,
    summary: Option<String>,
    status: Option<String>,
    description: Option<String>,
    html_link: Option<String>,
    #[serde(rename = "hangoutLink")]
    hangout_link: Option<String>,
    conference_data: Option<GoogleConferenceData>,
    location: Option<String>,
    start: GoogleEventDateTime,
    end: GoogleEventDateTime,
}

#[derive(Debug, Deserialize)]
struct GoogleConferenceData {
    entry_points: Option<Vec<GoogleConferenceEntryPoint>>,
}

#[derive(Debug, Deserialize)]
struct GoogleConferenceEntryPoint {
    uri: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GoogleCreateEventRequest<'a> {
    summary: &'a str,
    start: GoogleCreateEventDateTime<'a>,
    end: GoogleCreateEventDateTime<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    location: Option<&'a str>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GoogleCreateEventDateTime<'a> {
    date_time: &'a str,
    time_zone: &'a str,
}

#[derive(Debug, Deserialize)]
struct GoogleEventDateTime {
    #[serde(rename = "dateTime")]
    date_time: Option<String>,
    date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
    expires_in: i64,
    refresh_token: Option<String>,
}

#[plugin_fn]
pub fn plugin_init(_: String) -> FnResult<String> {
    ensure_sync_schedule().map_err(Error::msg)?;
    Ok(r#"{"status":"ok"}"#.to_string())
}

#[plugin_fn]
pub fn on_event(input: String) -> FnResult<String> {
    let event: Value = serde_json::from_str(&input)?;
    let event_name = event["event"].as_str().unwrap_or_default();
    let key = event["payload"]["key"].as_str().unwrap_or_default();

    if event_name == "schedule:fired" {
        if key == SYNC_SCHEDULE_KEY {
            let _ = refresh_snapshot(false);
        } else if let Some(event_id) = key.strip_prefix(REMINDER_SCHEDULE_PREFIX) {
            let _ = fire_event_reminder(event_id);
        }
    }
    Ok(r#"{"ok":true}"#.to_string())
}

#[plugin_fn]
pub fn tool_google_calendar_set_client_json(input: String) -> FnResult<String> {
    peekoo::log::info(&format!(
        "Received client.json upload, input length: {}",
        input.len()
    ));
    let payload: Value = serde_json::from_str(&input)?;
    let client_json = payload["clientJson"]
        .as_str()
        .ok_or_else(|| Error::msg("Missing clientJson"))?;
    peekoo::log::info(&format!(
        "Parsing client.json, length: {}",
        client_json.len()
    ));
    let credentials = parse_google_client_json(client_json).map_err(Error::msg)?;
    peekoo::log::info(&format!(
        "Parsed credentials, client_id: {}",
        credentials.client_id
    ));
    peekoo::secrets::set(CLIENT_ID_KEY, &credentials.client_id)?;
    peekoo::secrets::set(CLIENT_SECRET_KEY, &credentials.client_secret)?;
    peekoo::log::info("Client credentials saved successfully");
    Ok(r#"{"ok":true}"#.to_string())
}

#[plugin_fn]
pub fn tool_google_calendar_connect_start(_: String) -> FnResult<String> {
    let credentials = load_client_credentials().map_err(Error::msg)?;
    let result = peekoo::oauth::start(peekoo::oauth::StartRequest {
        provider_id: GOOGLE_PROVIDER_ID,
        authorize_url: GOOGLE_AUTHORIZE_URL,
        token_url: GOOGLE_TOKEN_URL,
        client_id: &credentials.client_id,
        client_secret: Some(&credentials.client_secret),
        redirect_uri: GOOGLE_REDIRECT_URI,
        scope: GOOGLE_SCOPES,
        authorize_params: vec![("access_type", "offline"), ("prompt", "consent")],
        token_params: vec![],
    })?;
    let mut state = load_calendar_state().map_err(Error::msg)?;
    state.oauth_flow_id = Some(result.flow_id.clone());
    save_calendar_state(&state).map_err(Error::msg)?;
    Ok(serde_json::to_string(&ConnectStartDto {
        flow_id: result.flow_id,
        authorize_url: result.authorize_url,
    })?)
}

#[plugin_fn]
pub fn tool_google_calendar_connect_status(input: String) -> FnResult<String> {
    let payload: FlowIdInput = serde_json::from_str(&input)?;
    let status = peekoo::oauth::status(&payload.flow_id)?;
    if status.status == "completed" {
        let bundle = TokenBundle {
            access_token: status
                .access_token
                .ok_or_else(|| Error::msg("Missing access token"))?,
            refresh_token: status.refresh_token,
            expires_at_epoch: status
                .expires_at
                .and_then(|value| value.parse::<i64>().ok()),
        };
        save_token_bundle(&bundle).map_err(Error::msg)?;
        let connected_account = fetch_account_profile(&bundle.access_token).map_err(Error::msg)?;
        save_connected_account(Some(&connected_account)).map_err(Error::msg)?;
        let mut state = load_calendar_state().map_err(Error::msg)?;
        state.oauth_flow_id = None;
        save_calendar_state(&state).map_err(Error::msg)?;
        refresh_snapshot(true).map_err(Error::msg)?;
    }

    Ok(serde_json::to_string(&GoogleCalendarOauthStatusDto {
        status: status.status,
        connected: load_token_bundle().map_err(Error::msg)?.is_some(),
        error: status.error,
    })?)
}

#[plugin_fn]
pub fn tool_google_calendar_disconnect(_: String) -> FnResult<String> {
    let _ = peekoo::secrets::delete(TOKEN_BUNDLE_KEY);
    let _ = peekoo::oauth::cancel(
        &load_calendar_state()
            .map_err(Error::msg)?
            .oauth_flow_id
            .unwrap_or_default(),
    );
    save_connected_account(None).map_err(Error::msg)?;
    save_calendar_state(&StoredCalendarState::default()).map_err(Error::msg)?;
    Ok(r#"{"ok":true}"#.to_string())
}

#[plugin_fn]
pub fn tool_google_calendar_refresh(_: String) -> FnResult<String> {
    let snapshot = refresh_snapshot(true).map_err(Error::msg)?;
    Ok(serde_json::to_string(&build_refresh_response(&snapshot))?)
}

#[plugin_fn]
pub fn tool_google_calendar_get_upcoming_events(_: String) -> FnResult<String> {
    let snapshot = agent_snapshot().map_err(Error::msg)?;
    Ok(serde_json::to_string(&build_agent_events_response(
        snapshot.status.last_sync_at,
        snapshot.status.connected,
        snapshot.upcoming.into_iter().map(bucket_to_event).collect(),
    ))?)
}

#[plugin_fn]
pub fn tool_google_calendar_get_daily_events(_: String) -> FnResult<String> {
    let snapshot = agent_snapshot().map_err(Error::msg)?;
    Ok(serde_json::to_string(&build_agent_events_response(
        snapshot.status.last_sync_at,
        snapshot.status.connected,
        snapshot.today.into_iter().map(bucket_to_event).collect(),
    ))?)
}

#[plugin_fn]
pub fn tool_google_calendar_get_weekly_events(_: String) -> FnResult<String> {
    let snapshot = agent_snapshot().map_err(Error::msg)?;
    Ok(serde_json::to_string(&build_agent_events_response(
        snapshot.status.last_sync_at,
        snapshot.status.connected,
        snapshot.week.into_iter().map(bucket_to_event).collect(),
    ))?)
}

#[plugin_fn]
pub fn tool_google_calendar_create_event_for_task(input: String) -> FnResult<String> {
    let payload: CreateTaskEventInput = serde_json::from_str(&input)?;
    let Some(mut bundle) = load_token_bundle().map_err(Error::msg)? else {
        return Err(Error::msg("Google Calendar is not connected.").into());
    };

    if token_expired_soon(&bundle) {
        bundle = refresh_token_bundle(&bundle).map_err(Error::msg)?;
        save_token_bundle(&bundle).map_err(Error::msg)?;
    }

    let created = create_google_event(&bundle.access_token, &payload).map_err(Error::msg)?;
    let mut state = load_calendar_state().map_err(Error::msg)?;
    upsert_task_link(&mut state, &payload.task_id, &created.id, "created");
    state.cached_events = upsert_cached_event(state.cached_events, created.clone());
    save_calendar_state(&state).map_err(Error::msg)?;
    schedule_event_reminders(&[created.clone()]).map_err(Error::msg)?;

    Ok(serde_json::to_string(&TaskCalendarEventActionResult {
        ok: true,
        event: to_calendar_event_dto(&created),
    })?)
}

#[plugin_fn]
pub fn tool_google_calendar_link_existing_event_to_task(input: String) -> FnResult<String> {
    let payload: LinkTaskEventInput = serde_json::from_str(&input)?;
    let mut state = load_calendar_state().map_err(Error::msg)?;
    let event = state
        .cached_events
        .iter()
        .find(|event| event.id == payload.event_id)
        .cloned()
        .ok_or_else(|| Error::msg("Event not found in current calendar snapshot"))?;

    let link_type = payload.link_type.as_deref().unwrap_or("linked");
    upsert_task_link(&mut state, &payload.task_id, &payload.event_id, link_type);
    save_calendar_state(&state).map_err(Error::msg)?;

    Ok(serde_json::to_string(&TaskCalendarEventActionResult {
        ok: true,
        event: to_calendar_event_dto(&event),
    })?)
}

#[plugin_fn]
pub fn tool_google_calendar_list_task_events(input: String) -> FnResult<String> {
    let payload: TaskIdInput = serde_json::from_str(&input)?;
    let state = load_calendar_state().map_err(Error::msg)?;
    let linked_ids = state
        .task_links
        .iter()
        .filter(|link| link.task_id == payload.task_id)
        .map(|link| link.event_id.clone())
        .collect::<std::collections::HashSet<_>>();
    let events = state
        .cached_events
        .iter()
        .filter(|event| linked_ids.contains(&event.id))
        .cloned()
        .collect::<Vec<_>>();

    Ok(serde_json::to_string(&TaskCalendarEventListResult {
        task_id: payload.task_id,
        count: events.len(),
        events: events.iter().map(to_calendar_event_dto).collect(),
    })?)
}

#[plugin_fn]
pub fn tool_google_calendar_unlink_task_event(input: String) -> FnResult<String> {
    let payload: LinkTaskEventInput = serde_json::from_str(&input)?;
    let mut state = load_calendar_state().map_err(Error::msg)?;
    state
        .task_links
        .retain(|link| !(link.task_id == payload.task_id && link.event_id == payload.event_id));
    save_calendar_state(&state).map_err(Error::msg)?;
    Ok(r#"{"ok":true}"#.to_string())
}

#[plugin_fn]
pub fn tool_google_calendar_update_calendar_selection(input: String) -> FnResult<String> {
    let payload: UpdateCalendarSelectionInput = serde_json::from_str(&input)?;
    let mut state = load_calendar_state().map_err(Error::msg)?;
    state.calendars = apply_calendar_selection(state.calendars, &payload.calendars);
    state.cached_events =
        filter_events_by_calendar_selection(state.cached_events, &state.calendars);
    save_calendar_state(&state).map_err(Error::msg)?;
    let snapshot = refresh_snapshot(true).map_err(Error::msg)?;
    Ok(serde_json::to_string(&snapshot)?)
}

#[plugin_fn]
pub fn data_panel_snapshot(_: String) -> FnResult<String> {
    let snapshot = panel_snapshot().map_err(Error::msg)?;
    Ok(serde_json::to_string(&snapshot)?)
}

fn panel_snapshot() -> Result<GoogleCalendarPanelDto, String> {
    let state = load_calendar_state()?;
    let client_credentials = load_client_credentials_optional()?;
    let now_iso = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let bucketed = bucket_events(&state.cached_events, &now_iso, DEFAULT_UPCOMING_LIMIT)?;

    Ok(GoogleCalendarPanelDto {
        status: GoogleCalendarStatusDto {
            connected: load_token_bundle()?.is_some(),
            client_configured: client_credentials.is_some(),
            client_json_uploaded: client_credentials.is_some(),
            effective_client_id: client_credentials
                .as_ref()
                .map(|value| value.client_id.clone())
                .unwrap_or_default(),
            connected_account: load_connected_account()?,
            last_sync_at: state.last_sync_at,
            last_error: state.last_error,
        },
        calendars: state.calendars.clone(),
        upcoming: bucketed.upcoming,
        today: bucketed.today,
        week: bucketed.week,
        event_link_statuses: build_event_link_statuses(&state.task_links),
    })
}

fn build_event_link_statuses(links: &[TaskCalendarLink]) -> Vec<EventLinkStatus> {
    links
        .iter()
        .map(|link| EventLinkStatus {
            event_id: link.event_id.clone(),
            task_id: link.task_id.clone(),
            status: link.link_type.clone(),
        })
        .collect()
}

fn agent_snapshot() -> Result<GoogleCalendarPanelDto, String> {
    let snapshot = refresh_snapshot(false)?;
    if snapshot.status.connected && snapshot.status.last_sync_at.is_none() {
        return refresh_snapshot(true);
    }
    Ok(snapshot)
}

fn build_agent_events_response(
    last_sync_at: Option<String>,
    connected: bool,
    events: Vec<CalendarEvent>,
) -> GoogleCalendarAgentEventsDto {
    GoogleCalendarAgentEventsDto {
        connected,
        last_sync_at,
        count: events.len(),
        events: events.iter().map(to_calendar_event_dto).collect(),
    }
}

fn build_refresh_response(snapshot: &GoogleCalendarPanelDto) -> GoogleCalendarRefreshDto {
    GoogleCalendarRefreshDto {
        connected: snapshot.status.connected,
        last_sync_at: snapshot.status.last_sync_at.clone(),
        upcoming_count: snapshot.upcoming.len(),
        daily_count: snapshot.today.len(),
        weekly_count: snapshot.week.len(),
    }
}

fn to_calendar_event_dto(event: &CalendarEvent) -> CalendarEventDto {
    CalendarEventDto {
        id: event.id.clone(),
        title: event.title.clone(),
        start_at: event.start_at.clone(),
        end_at: event.end_at.clone(),
        all_day: event.all_day,
        location: event.location.clone(),
        description: event.description.clone(),
        calendar_name: event.calendar_name.clone(),
        html_link: event.html_link.clone(),
        meeting_url: event.meeting_url.clone(),
        status: event.status.clone(),
    }
}

fn bucket_to_event(bucket: CalendarEventBucket) -> CalendarEvent {
    CalendarEvent {
        id: bucket.id,
        title: bucket.title,
        start_at: bucket.start_at.clone(),
        end_at: bucket.end_at,
        all_day: bucket.all_day,
        location: bucket.location,
        description: bucket.description,
        calendar_id: String::new(),
        calendar_name: bucket.calendar_name,
        html_link: bucket.html_link,
        meeting_url: bucket.meeting_url,
        status: "confirmed".to_string(),
    }
}

fn refresh_snapshot(force: bool) -> Result<GoogleCalendarPanelDto, String> {
    ensure_sync_schedule()?;
    let Some(mut bundle) = load_token_bundle()? else {
        return panel_snapshot();
    };

    if !force {
        let state = load_calendar_state()?;
        if let Some(last_sync_at) = state.last_sync_at.as_deref() {
            if let Ok(last_sync) = chrono::DateTime::parse_from_rfc3339(last_sync_at) {
                if (Utc::now() - last_sync.with_timezone(&Utc)).num_seconds()
                    < DEFAULT_REFRESH_INTERVAL_SECS
                {
                    return panel_snapshot();
                }
            }
        }
    }

    if token_expired_soon(&bundle) {
        bundle = refresh_token_bundle(&bundle)?;
        save_token_bundle(&bundle)?;
    }

    let mut state = load_calendar_state()?;
    match fetch_events(&bundle.access_token, &state.calendars) {
        Ok((events, calendars, errors)) => {
            state.cached_events = events;
            state.calendars = calendars;
            state.last_sync_at = Some(Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true));
            state.last_error = if errors.is_empty() {
                None
            } else {
                Some(format!(
                    "Partial calendar sync failure: {}",
                    errors.join(" | ")
                ))
            };
            notify_due_events(&mut state)?;
        }
        Err(err) => {
            state.last_error = Some(err);
        }
    }
    save_calendar_state(&state)?;
    panel_snapshot()
}

fn ensure_sync_schedule() -> Result<(), String> {
    peekoo::schedule::set(
        SYNC_SCHEDULE_KEY,
        DEFAULT_REFRESH_INTERVAL_SECS as u64,
        true,
        None,
    )
    .map_err(|e| e.to_string())
}

fn load_client_credentials() -> Result<GoogleClientCredentials, String> {
    load_client_credentials_optional()?
        .ok_or_else(|| "Upload your Google OAuth client.json file first.".to_string())
}

fn load_client_credentials_optional() -> Result<Option<GoogleClientCredentials>, String> {
    let Some(client_id) = peekoo::secrets::get(CLIENT_ID_KEY).map_err(|e| e.to_string())? else {
        return Ok(None);
    };
    let Some(client_secret) = peekoo::secrets::get(CLIENT_SECRET_KEY).map_err(|e| e.to_string())?
    else {
        return Ok(None);
    };

    Ok(Some(GoogleClientCredentials {
        client_id,
        client_secret,
    }))
}

fn load_token_bundle() -> Result<Option<TokenBundle>, String> {
    let Some(raw) = peekoo::secrets::get(TOKEN_BUNDLE_KEY).map_err(|e| e.to_string())? else {
        return Ok(None);
    };
    serde_json::from_str(&raw)
        .map(Some)
        .map_err(|e| e.to_string())
}

fn save_token_bundle(bundle: &TokenBundle) -> Result<(), String> {
    let raw = serde_json::to_string(bundle).map_err(|e| e.to_string())?;
    peekoo::secrets::set(TOKEN_BUNDLE_KEY, &raw).map_err(|e| e.to_string())
}

fn load_connected_account() -> Result<Option<GoogleAccountProfile>, String> {
    let Some(raw) =
        peekoo::state::get::<String>(CONNECTED_ACCOUNT_KEY).map_err(|e| e.to_string())?
    else {
        return Ok(None);
    };
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}

fn save_connected_account(profile: Option<&GoogleAccountProfile>) -> Result<(), String> {
    match profile {
        Some(profile) => {
            let raw = serde_json::to_string(profile).map_err(|e| e.to_string())?;
            peekoo::state::set(CONNECTED_ACCOUNT_KEY, &raw).map_err(|e| e.to_string())
        }
        None => peekoo::state::delete(CONNECTED_ACCOUNT_KEY).map_err(|e| e.to_string()),
    }
}

fn load_calendar_state() -> Result<StoredCalendarState, String> {
    Ok(peekoo::state::get(STATE_KEY)
        .map_err(|e| e.to_string())?
        .unwrap_or_default())
}

fn save_calendar_state(state: &StoredCalendarState) -> Result<(), String> {
    peekoo::state::set(STATE_KEY, state).map_err(|e| e.to_string())
}

fn upsert_task_link(
    state: &mut StoredCalendarState,
    task_id: &str,
    event_id: &str,
    link_type: &str,
) {
    if let Some(existing) = state
        .task_links
        .iter_mut()
        .find(|link| link.task_id == task_id && link.event_id == event_id)
    {
        existing.link_type = link_type.to_string();
        return;
    }
    state.task_links.push(TaskCalendarLink {
        task_id: task_id.to_string(),
        event_id: event_id.to_string(),
        link_type: link_type.to_string(),
    });
}

fn upsert_cached_event(mut events: Vec<CalendarEvent>, event: CalendarEvent) -> Vec<CalendarEvent> {
    if let Some(existing) = events.iter_mut().find(|existing| existing.id == event.id) {
        *existing = event;
    } else {
        events.push(event);
    }
    events
}

fn refresh_token_bundle(bundle: &TokenBundle) -> Result<TokenBundle, String> {
    let credentials = load_client_credentials()?;
    let refresh_token = bundle.refresh_token.as_deref().ok_or_else(|| {
        "Google Calendar refresh token is missing. Reconnect the account.".to_string()
    })?;
    let body = format!(
        "grant_type=refresh_token&client_id={}&refresh_token={}&client_secret={}",
        percent_encode_component(&credentials.client_id),
        percent_encode_component(refresh_token),
        percent_encode_component(&credentials.client_secret),
    );
    let response = peekoo::http::request(peekoo::http::Request {
        method: "POST",
        url: "https://oauth2.googleapis.com/token",
        headers: vec![
            ("Content-Type", "application/x-www-form-urlencoded"),
            ("Accept", "application/json"),
            ("User-Agent", "Peekoo-Desktop/0.1.0"),
        ],
        body: Some(&body),
    })
    .map_err(|e| e.to_string())?;
    if response.status >= 400 {
        return Err(format!(
            "Google Calendar token refresh failed ({}): {}",
            response.status, response.body
        ));
    }
    let refreshed: GoogleTokenResponse =
        serde_json::from_str(&response.body).map_err(|e| e.to_string())?;
    Ok(TokenBundle {
        access_token: refreshed.access_token,
        refresh_token: refreshed
            .refresh_token
            .or_else(|| bundle.refresh_token.clone()),
        expires_at_epoch: Some(Utc::now().timestamp() + refreshed.expires_in),
    })
}

fn fetch_events(
    access_token: &str,
    stored_calendars: &[StoredGoogleCalendar],
) -> Result<(Vec<CalendarEvent>, Vec<StoredGoogleCalendar>, Vec<String>), String> {
    let calendar_entries = fetch_google_calendar_list(access_token)?;
    let calendars = merge_calendar_configs(calendar_entries, stored_calendars);
    let enabled_calendars = enabled_calendars(&calendars);

    let (time_min, time_max) = default_sync_window();
    let mut events = Vec::new();
    let mut errors = Vec::new();

    for calendar in enabled_calendars {
        match fetch_calendar_events(access_token, &calendar, &time_min, &time_max) {
            Ok(mut calendar_events) => events.append(&mut calendar_events),
            Err(error) => {
                peekoo::log::error(&error);
                errors.push(error);
            }
        }
    }

    events.sort_by(|left, right| left.start_at.cmp(&right.start_at));
    Ok((events, calendars, errors))
}

fn default_sync_window() -> (DateTime<Utc>, DateTime<Utc>) {
    let now = Utc::now();
    let time_min = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .expect("midnight is valid")
        .and_utc();
    let time_max = now
        .date_naive()
        .checked_add_days(Days::new(7))
        .unwrap_or(now.date_naive())
        .and_hms_opt(23, 59, 59)
        .expect("end of day is valid")
        .and_utc();
    (time_min, time_max)
}

fn fetch_google_calendar_list(access_token: &str) -> Result<Vec<GoogleCalendarListEntry>, String> {
    let response = peekoo::http::request(peekoo::http::Request {
        method: "GET",
        url: "https://www.googleapis.com/calendar/v3/users/me/calendarList",
        headers: vec![
            ("Authorization", &format!("Bearer {access_token}")),
            ("User-Agent", "Peekoo-Desktop/0.1.0"),
        ],
        body: None,
    })
    .map_err(|e| e.to_string())?;

    if response.status >= 400 {
        let err_msg = format!(
            "Google Calendar calendar list fetch failed ({}): {}",
            response.status, response.body
        );
        peekoo::log::error(&err_msg);
        return Err(err_msg);
    }

    parse_google_calendar_list(&response.body)
}

fn fetch_calendar_events(
    access_token: &str,
    calendar: &StoredGoogleCalendar,
    time_min: &DateTime<Utc>,
    time_max: &DateTime<Utc>,
) -> Result<Vec<CalendarEvent>, String> {
    let url = format!(
        "https://www.googleapis.com/calendar/v3/calendars/{}/events?singleEvents=true&orderBy=startTime&timeMin={}&timeMax={}",
        percent_encode_component(&calendar.id),
        percent_encode_component(&time_min.to_rfc3339_opts(SecondsFormat::Secs, true)),
        percent_encode_component(&time_max.to_rfc3339_opts(SecondsFormat::Secs, true)),
    );
    let response = peekoo::http::request(peekoo::http::Request {
        method: "GET",
        url: &url,
        headers: vec![
            ("Authorization", &format!("Bearer {access_token}")),
            ("User-Agent", "Peekoo-Desktop/0.1.0"),
        ],
        body: None,
    })
    .map_err(|e| e.to_string())?;
    if response.status >= 400 {
        return Err(format!(
            "Google Calendar fetch failed for '{}' ({}): {}",
            calendar.name, response.status, response.body
        ));
    }
    let parsed: GoogleEventsResponse =
        serde_json::from_str(&response.body).map_err(|e| e.to_string())?;
    parsed
        .items
        .into_iter()
        .map(|event| normalize_event(event, &calendar.id, &calendar.name))
        .collect()
}

fn create_google_event(
    access_token: &str,
    input: &CreateTaskEventInput,
) -> Result<CalendarEvent, String> {
    let payload = GoogleCreateEventRequest {
        summary: &input.title,
        start: GoogleCreateEventDateTime {
            date_time: &input.start_at,
            time_zone: "UTC",
        },
        end: GoogleCreateEventDateTime {
            date_time: &input.end_at,
            time_zone: "UTC",
        },
        description: input.description.as_deref(),
        location: input.location.as_deref(),
    };
    let body = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
    let response = peekoo::http::request(peekoo::http::Request {
        method: "POST",
        url: "https://www.googleapis.com/calendar/v3/calendars/primary/events?conferenceDataVersion=1",
        headers: vec![
            ("Authorization", &format!("Bearer {access_token}")),
            ("Content-Type", "application/json"),
            ("Accept", "application/json"),
            ("User-Agent", "Peekoo-Desktop/0.1.0"),
        ],
        body: Some(&body),
    })
    .map_err(|e| e.to_string())?;

    if response.status >= 400 {
        return Err(format!(
            "Google Calendar create event failed ({}): {}",
            response.status, response.body
        ));
    }

    let event: GoogleEventItem = serde_json::from_str(&response.body).map_err(|e| e.to_string())?;
    normalize_event(event, "primary", "Primary")
}

fn fetch_account_profile(access_token: &str) -> Result<GoogleAccountProfile, String> {
    let response = peekoo::http::request(peekoo::http::Request {
        method: "GET",
        url: "https://www.googleapis.com/oauth2/v2/userinfo",
        headers: vec![
            ("Authorization", &format!("Bearer {access_token}")),
            ("User-Agent", "Peekoo-Desktop/0.1.0"),
        ],
        body: None,
    })
    .map_err(|e| e.to_string())?;
    if response.status >= 400 {
        return Err(format!(
            "Google account profile fetch failed ({}): {}",
            response.status, response.body
        ));
    }
    parse_google_account_profile(&response.body)
}

fn notify_due_events(state: &mut StoredCalendarState) -> Result<(), String> {
    schedule_event_reminders(&state.cached_events)?;
    Ok(())
}

fn fire_event_reminder(event_id: &str) -> Result<(), String> {
    let state = load_calendar_state()?;
    let Some(event) = state.cached_events.iter().find(|e| e.id == event_id) else {
        return Ok(());
    };
    if event.all_day {
        return Ok(());
    }
    let meeting_url = event.meeting_url.as_deref().or(event.html_link.as_deref());
    let _ = peekoo::notify::send_full(
        &event.title,
        "Starts now",
        meeting_url,
        meeting_url.map(|_| "Join meeting"),
        Some("panel-google-calendar"),
    );
    Ok(())
}

fn schedule_event_reminders(events: &[CalendarEvent]) -> Result<(), String> {
    let now_iso = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let delays = pending_reminder_delays(events, &now_iso)?;
    for (event_id, delay_secs) in delays {
        let key = reminder_schedule_key(&event_id);
        let _ = peekoo::schedule::set(&key, delay_secs.max(1), false, Some(delay_secs));
    }
    Ok(())
}

fn token_expired_soon(bundle: &TokenBundle) -> bool {
    bundle
        .expires_at_epoch
        .map(|expires_at| expires_at <= Utc::now().timestamp() + 60)
        .unwrap_or(false)
}

fn parse_google_client_json(raw: &str) -> Result<GoogleClientCredentials, String> {
    let parsed: GoogleClientJson = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    if let Some(installed) = parsed.installed {
        return Ok(GoogleClientCredentials {
            client_id: installed.client_id,
            client_secret: installed.client_secret,
        });
    }
    if let Some(web) = parsed.web {
        return Ok(GoogleClientCredentials {
            client_id: web.client_id,
            client_secret: web.client_secret,
        });
    }
    Err("Google client json must contain an 'installed' or 'web' object".to_string())
}

fn parse_google_calendar_list(raw: &str) -> Result<Vec<GoogleCalendarListEntry>, String> {
    let payload: GoogleCalendarListResponse =
        serde_json::from_str(raw).map_err(|e| e.to_string())?;
    Ok(payload
        .items
        .into_iter()
        .filter(is_readable_calendar)
        .collect())
}

fn is_readable_calendar(entry: &GoogleCalendarListEntry) -> bool {
    !matches!(
        entry.access_role.as_deref(),
        Some("freeBusyReader") | Some("none")
    )
}

fn merge_calendar_configs(
    entries: Vec<GoogleCalendarListEntry>,
    stored_calendars: &[StoredGoogleCalendar],
) -> Vec<StoredGoogleCalendar> {
    entries
        .into_iter()
        .map(|entry| {
            let existing = stored_calendars
                .iter()
                .find(|calendar| calendar.id == entry.id);
            StoredGoogleCalendar {
                id: entry.id,
                name: entry
                    .summary_override
                    .or(entry.summary)
                    .unwrap_or_else(|| "Untitled calendar".to_string()),
                primary: entry.primary.unwrap_or(false),
                access_role: entry.access_role.unwrap_or_else(|| "reader".to_string()),
                enabled: existing.map(|calendar| calendar.enabled).unwrap_or(true),
            }
        })
        .collect()
}

fn apply_calendar_selection(
    calendars: Vec<StoredGoogleCalendar>,
    selected: &[CalendarSelectionInput],
) -> Vec<StoredGoogleCalendar> {
    calendars
        .into_iter()
        .map(|mut calendar| {
            if let Some(selection) = selected
                .iter()
                .find(|selection| selection.id == calendar.id)
            {
                calendar.enabled = selection.enabled;
            }
            calendar
        })
        .collect()
}

fn enabled_calendars(calendars: &[StoredGoogleCalendar]) -> Vec<StoredGoogleCalendar> {
    calendars
        .iter()
        .filter(|calendar| calendar.enabled)
        .cloned()
        .collect()
}

fn filter_events_by_calendar_selection(
    events: Vec<CalendarEvent>,
    calendars: &[StoredGoogleCalendar],
) -> Vec<CalendarEvent> {
    events
        .into_iter()
        .filter(|event| {
            calendars
                .iter()
                .find(|calendar| calendar.id == event.calendar_id)
                .map(|calendar| calendar.enabled)
                .unwrap_or(false)
        })
        .collect()
}

fn parse_google_account_profile(raw: &str) -> Result<GoogleAccountProfile, String> {
    let payload: GoogleAccountProfilePayload =
        serde_json::from_str(raw).map_err(|e| e.to_string())?;
    let email = payload
        .email
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Google account profile is missing email".to_string())?;

    Ok(GoogleAccountProfile {
        email,
        name: payload
            .name
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        picture: payload
            .picture
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
    })
}

fn normalize_event(
    event: GoogleEventItem,
    calendar_id: &str,
    calendar_name: &str,
) -> Result<CalendarEvent, String> {
    let (start_at, all_day) = normalize_event_time(&event.start)?;
    let (end_at, _) = normalize_event_time(&event.end)?;
    let meeting_url = event
        .hangout_link
        .clone()
        .or_else(|| conference_meeting_url(event.conference_data.as_ref()));
    Ok(CalendarEvent {
        id: event.id,
        title: event
            .summary
            .unwrap_or_else(|| "Untitled event".to_string()),
        start_at,
        end_at,
        all_day,
        location: event.location,
        description: event.description,
        calendar_id: calendar_id.to_string(),
        calendar_name: calendar_name.to_string(),
        html_link: event.html_link,
        meeting_url,
        status: event.status.unwrap_or_else(|| "confirmed".to_string()),
    })
}

fn conference_meeting_url(conference_data: Option<&GoogleConferenceData>) -> Option<String> {
    conference_data
        .and_then(|data| data.entry_points.as_ref())
        .and_then(|entry_points| {
            entry_points
                .iter()
                .find_map(|entry| entry.uri.as_ref().map(ToString::to_string))
        })
}

fn normalize_event_time(value: &GoogleEventDateTime) -> Result<(String, bool), String> {
    if let Some(date_time) = value.date_time.clone() {
        return Ok((date_time, false));
    }
    if let Some(date) = value.date.clone() {
        return Ok((date, true));
    }
    Err("Google Calendar event time is missing".to_string())
}

fn bucket_events(
    events: &[CalendarEvent],
    now_iso: &str,
    upcoming_limit: usize,
) -> Result<BucketedCalendarEvents, String> {
    let now = parse_datetime(now_iso)?;
    let today = now.date_naive();
    let week_start = start_of_week(today);
    let week_end = week_start + Duration::days(7);

    let mut future_events: Vec<_> = events
        .iter()
        .filter_map(|event| classify_event(event, today).ok())
        .filter(|event| event.end >= now)
        .collect();
    future_events.sort_by_key(|event| event.start);

    Ok(BucketedCalendarEvents {
        upcoming: future_events
            .iter()
            .take(upcoming_limit)
            .map(|event| event.bucket.clone())
            .collect(),
        today: future_events
            .iter()
            .filter(|event| event.start.date_naive() == today || event.end.date_naive() == today)
            .map(|event| event.bucket.clone())
            .collect(),
        week: future_events
            .iter()
            .filter(|event| {
                let event_day = event.start.date_naive();
                event_day > today && event_day >= week_start && event_day < week_end
            })
            .map(|event| event.bucket.clone())
            .collect(),
    })
}

fn reminder_schedule_key(event_id: &str) -> String {
    format!("{REMINDER_SCHEDULE_PREFIX}{event_id}")
}

/// Returns `(event_id, delay_secs)` for every future timed event.
fn pending_reminder_delays(
    events: &[CalendarEvent],
    now_iso: &str,
) -> Result<Vec<(String, u64)>, String> {
    let now = parse_datetime(now_iso)?;
    let mut out = Vec::new();
    for event in events {
        if event.all_day {
            continue;
        }
        let start = parse_datetime(&event.start_at)?;
        if start <= now {
            continue;
        }
        let delay_secs = (start - now).num_seconds().max(0) as u64;
        out.push((event.id.clone(), delay_secs));
    }
    Ok(out)
}

#[derive(Clone)]
struct ClassifiedEvent {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    bucket: CalendarEventBucket,
}

fn classify_event(event: &CalendarEvent, today: NaiveDate) -> Result<ClassifiedEvent, String> {
    let start = parse_event_start(event)?;
    let end = parse_event_end(event, today)?;
    Ok(ClassifiedEvent {
        start,
        end,
        bucket: CalendarEventBucket {
            id: event.id.clone(),
            title: event.title.clone(),
            start_at: event.start_at.clone(),
            end_at: event.end_at.clone(),
            all_day: event.all_day,
            location: event.location.clone(),
            description: event.description.clone(),
            calendar_name: event.calendar_name.clone(),
            html_link: event.html_link.clone(),
            meeting_url: event.meeting_url.clone(),
        },
    })
}

fn parse_event_start(event: &CalendarEvent) -> Result<DateTime<Utc>, String> {
    if event.all_day {
        let day =
            NaiveDate::parse_from_str(&event.start_at, "%Y-%m-%d").map_err(|e| e.to_string())?;
        return Ok(day
            .and_hms_opt(0, 0, 0)
            .expect("midnight is valid")
            .and_utc());
    }
    parse_datetime(&event.start_at)
}

fn parse_event_end(event: &CalendarEvent, today: NaiveDate) -> Result<DateTime<Utc>, String> {
    if event.all_day {
        let day =
            NaiveDate::parse_from_str(&event.end_at, "%Y-%m-%d").map_err(|e| e.to_string())?;
        return Ok(day
            .and_hms_opt(0, 0, 0)
            .expect("midnight is valid")
            .and_utc());
    }
    let end = parse_datetime(&event.end_at)?;
    if end.date_naive().year() < today.year() - 10 {
        return Err("Calendar event end time is unexpectedly old".to_string());
    }
    Ok(end)
}

fn parse_datetime(value: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|e| e.to_string())
}

fn start_of_week(day: NaiveDate) -> NaiveDate {
    let days_from_monday = match day.weekday() {
        Weekday::Mon => 0,
        Weekday::Tue => 1,
        Weekday::Wed => 2,
        Weekday::Thu => 3,
        Weekday::Fri => 4,
        Weekday::Sat => 5,
        Weekday::Sun => 6,
    };
    day - Duration::days(days_from_monday)
}

fn percent_encode_component(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(*byte as char)
            }
            b' ' => out.push_str("%20"),
            other => {
                let _ = std::fmt::Write::write_fmt(&mut out, format_args!("%{other:02X}"));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_google_client_json_from_installed_credentials() {
        let parsed = parse_google_client_json(
            r#"{
                "installed": {
                    "client_id": "client-id",
                    "client_secret": "client-secret"
                }
            }"#,
        )
        .expect("client json parses");

        assert_eq!(parsed.client_id, "client-id");
        assert_eq!(parsed.client_secret, "client-secret");
    }

    #[test]
    fn buckets_upcoming_today_and_week_events() {
        let events = vec![
            CalendarEvent {
                id: "a".to_string(),
                title: "Daily standup".to_string(),
                start_at: "2026-03-19T10:00:00Z".to_string(),
                end_at: "2026-03-19T10:30:00Z".to_string(),
                all_day: false,
                location: None,
                description: None,
                calendar_id: "primary".to_string(),
                calendar_name: "Primary".to_string(),
                html_link: None,
                meeting_url: None,
                status: "confirmed".to_string(),
            },
            CalendarEvent {
                id: "b".to_string(),
                title: "Planning".to_string(),
                start_at: "2026-03-20T14:00:00Z".to_string(),
                end_at: "2026-03-20T15:00:00Z".to_string(),
                all_day: false,
                location: None,
                description: None,
                calendar_id: "primary".to_string(),
                calendar_name: "Primary".to_string(),
                html_link: None,
                meeting_url: None,
                status: "confirmed".to_string(),
            },
        ];

        let bucketed = bucket_events(&events, "2026-03-19T09:00:00Z", 5).expect("bucket events");

        assert_eq!(bucketed.upcoming.len(), 2);
        assert_eq!(bucketed.today.len(), 1);
        assert_eq!(bucketed.week.len(), 1);
    }

    #[test]
    fn flow_id_input_accepts_snake_case_payload_from_panel() {
        let parsed: FlowIdInput =
            serde_json::from_str(r#"{"flow_id":"flow-123"}"#).expect("flow id input parses");

        assert_eq!(parsed.flow_id, "flow-123");
    }

    #[test]
    fn agenda_tool_response_includes_metadata_and_events() {
        let response = build_agent_events_response(
            Some("2026-03-19T17:06:12Z".to_string()),
            true,
            vec![CalendarEvent {
                id: "evt_1".to_string(),
                title: "Design review".to_string(),
                start_at: "2026-03-20T09:30:00Z".to_string(),
                end_at: "2026-03-20T10:00:00Z".to_string(),
                all_day: false,
                location: Some("Zoom".to_string()),
                description: Some("Team design review".to_string()),
                calendar_id: "primary".to_string(),
                calendar_name: "Primary".to_string(),
                html_link: Some("https://example.com".to_string()),
                meeting_url: Some("https://meet.google.com/abc-defg-hij".to_string()),
                status: "confirmed".to_string(),
            }],
        );

        assert!(response.connected);
        assert_eq!(
            response.last_sync_at.as_deref(),
            Some("2026-03-19T17:06:12Z")
        );
        assert_eq!(response.count, 1);
        assert_eq!(response.events[0].title, "Design review");
        assert_eq!(
            response.events[0].meeting_url.as_deref(),
            Some("https://meet.google.com/abc-defg-hij")
        );
    }

    #[test]
    fn refresh_response_reports_bucket_counts() {
        let response = build_refresh_response(&GoogleCalendarPanelDto {
            status: GoogleCalendarStatusDto {
                connected: true,
                client_configured: true,
                client_json_uploaded: true,
                effective_client_id: "client-id".to_string(),
                connected_account: None,
                last_sync_at: Some("2026-03-19T17:06:12Z".to_string()),
                last_error: None,
            },
            calendars: vec![],
            upcoming: vec![sample_bucket("evt_1")],
            today: vec![sample_bucket("evt_2"), sample_bucket("evt_3")],
            week: vec![sample_bucket("evt_4")],
            event_link_statuses: vec![],
        });

        assert!(response.connected);
        assert_eq!(response.upcoming_count, 1);
        assert_eq!(response.daily_count, 2);
        assert_eq!(response.weekly_count, 1);
    }

    fn sample_bucket(id: &str) -> CalendarEventBucket {
        CalendarEventBucket {
            id: id.to_string(),
            title: "Event".to_string(),
            start_at: "2026-03-20T09:30:00Z".to_string(),
            end_at: "2026-03-20T10:00:00Z".to_string(),
            all_day: false,
            location: None,
            description: None,
            calendar_name: "Primary".to_string(),
            html_link: None,
            meeting_url: None,
        }
    }

    #[test]
    fn merge_calendar_configs_defaults_new_readable_calendars_to_enabled() {
        let calendars = merge_calendar_configs(
            vec![GoogleCalendarListEntry {
                id: "team@example.com".to_string(),
                summary: Some("Team".to_string()),
                summary_override: None,
                primary: Some(false),
                access_role: Some("reader".to_string()),
            }],
            &[],
        );

        assert_eq!(calendars.len(), 1);
        assert_eq!(calendars[0].id, "team@example.com");
        assert_eq!(calendars[0].name, "Team");
        assert!(calendars[0].enabled);
    }

    #[test]
    fn merge_calendar_configs_preserves_existing_enabled_preference() {
        let calendars = merge_calendar_configs(
            vec![GoogleCalendarListEntry {
                id: "team@example.com".to_string(),
                summary: Some("Team".to_string()),
                summary_override: None,
                primary: Some(false),
                access_role: Some("reader".to_string()),
            }],
            &[StoredGoogleCalendar {
                id: "team@example.com".to_string(),
                name: "Old Team".to_string(),
                primary: false,
                access_role: "reader".to_string(),
                enabled: false,
            }],
        );

        assert_eq!(calendars.len(), 1);
        assert!(!calendars[0].enabled);
        assert_eq!(calendars[0].name, "Team");
    }

    #[test]
    fn normalize_event_extracts_hangout_link_and_calendar_name() {
        let event = normalize_event(
            GoogleEventItem {
                id: "evt_2".to_string(),
                summary: Some("Weekly sync".to_string()),
                status: Some("confirmed".to_string()),
                description: Some("Discuss roadmap".to_string()),
                html_link: Some("https://calendar.google.com/event?eid=abc".to_string()),
                hangout_link: Some("https://meet.google.com/room".to_string()),
                conference_data: None,
                location: None,
                start: GoogleEventDateTime {
                    date_time: Some("2026-03-20T10:00:00Z".to_string()),
                    date: None,
                },
                end: GoogleEventDateTime {
                    date_time: Some("2026-03-20T10:30:00Z".to_string()),
                    date: None,
                },
            },
            "team@example.com",
            "Team Calendar",
        )
        .expect("event normalizes");

        assert_eq!(event.calendar_id, "team@example.com");
        assert_eq!(event.calendar_name, "Team Calendar");
        assert_eq!(
            event.meeting_url.as_deref(),
            Some("https://meet.google.com/room")
        );
    }

    #[test]
    fn filters_out_non_readable_calendars() {
        let calendars = parse_google_calendar_list(
            r#"{
                "items": [
                    {
                        "id": "team@example.com",
                        "summary": "Team",
                        "accessRole": "reader"
                    },
                    {
                        "id": "busy@example.com",
                        "summary": "Busy",
                        "accessRole": "freeBusyReader"
                    }
                ]
            }"#,
        )
        .expect("calendar list parses");

        assert_eq!(calendars.len(), 1);
        assert_eq!(calendars[0].id, "team@example.com");
    }

    #[test]
    fn apply_calendar_selection_updates_enabled_flags() {
        let updated = apply_calendar_selection(
            vec![
                StoredGoogleCalendar {
                    id: "primary".to_string(),
                    name: "Primary".to_string(),
                    primary: true,
                    access_role: "owner".to_string(),
                    enabled: true,
                },
                StoredGoogleCalendar {
                    id: "team@example.com".to_string(),
                    name: "Team".to_string(),
                    primary: false,
                    access_role: "reader".to_string(),
                    enabled: true,
                },
            ],
            &[CalendarSelectionInput {
                id: "team@example.com".to_string(),
                enabled: false,
            }],
        );

        assert!(updated[0].enabled);
        assert!(!updated[1].enabled);
    }

    #[test]
    fn enabled_calendars_only_returns_enabled_entries() {
        let calendars = enabled_calendars(&[
            StoredGoogleCalendar {
                id: "primary".to_string(),
                name: "Primary".to_string(),
                primary: true,
                access_role: "owner".to_string(),
                enabled: true,
            },
            StoredGoogleCalendar {
                id: "team@example.com".to_string(),
                name: "Team".to_string(),
                primary: false,
                access_role: "reader".to_string(),
                enabled: false,
            },
        ]);

        assert_eq!(calendars.len(), 1);
        assert_eq!(calendars[0].id, "primary");
    }

    #[test]
    fn filter_events_by_calendar_selection_uses_calendar_id() {
        let calendars = vec![
            StoredGoogleCalendar {
                id: "primary".to_string(),
                name: "Shared Name".to_string(),
                primary: true,
                access_role: "owner".to_string(),
                enabled: true,
            },
            StoredGoogleCalendar {
                id: "team@example.com".to_string(),
                name: "Shared Name".to_string(),
                primary: false,
                access_role: "reader".to_string(),
                enabled: false,
            },
        ];
        let events = vec![
            CalendarEvent {
                id: "evt-primary".to_string(),
                title: "Primary event".to_string(),
                start_at: "2026-03-20T09:30:00Z".to_string(),
                end_at: "2026-03-20T10:00:00Z".to_string(),
                all_day: false,
                location: None,
                description: None,
                calendar_id: "primary".to_string(),
                calendar_name: "Shared Name".to_string(),
                html_link: None,
                meeting_url: None,
                status: "confirmed".to_string(),
            },
            CalendarEvent {
                id: "evt-team".to_string(),
                title: "Team event".to_string(),
                start_at: "2026-03-20T11:30:00Z".to_string(),
                end_at: "2026-03-20T12:00:00Z".to_string(),
                all_day: false,
                location: None,
                description: None,
                calendar_id: "team@example.com".to_string(),
                calendar_name: "Shared Name".to_string(),
                html_link: None,
                meeting_url: None,
                status: "confirmed".to_string(),
            },
        ];

        let filtered = filter_events_by_calendar_selection(events, &calendars);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "evt-primary");
    }

    #[test]
    fn stored_calendar_state_deserializes_legacy_events_without_calendar_id() {
        let state: StoredCalendarState = serde_json::from_str(
            r#"{
                "lastSyncAt": null,
                "lastError": null,
                "cachedEvents": [
                    {
                        "id": "evt-1",
                        "title": "Legacy event",
                        "startAt": "2026-03-20T09:30:00Z",
                        "endAt": "2026-03-20T10:00:00Z",
                        "allDay": false,
                        "location": null,
                        "description": null,
                        "calendarName": "Primary",
                        "htmlLink": null,
                        "meetingUrl": null,
                        "status": "confirmed"
                    }
                ],
                "notifiedEventIds": [],
                "taskLinks": [],
                "calendars": [],
                "oauthFlowId": null
            }"#,
        )
        .expect("legacy state deserializes");

        assert_eq!(state.cached_events.len(), 1);
        assert_eq!(state.cached_events[0].calendar_id, "");
    }

    #[test]
    fn calendar_event_dto_omits_internal_calendar_id() {
        let raw = serde_json::to_string(&to_calendar_event_dto(&CalendarEvent {
            id: "evt-1".to_string(),
            title: "Event".to_string(),
            start_at: "2026-03-20T09:30:00Z".to_string(),
            end_at: "2026-03-20T10:00:00Z".to_string(),
            all_day: false,
            location: None,
            description: None,
            calendar_id: "primary".to_string(),
            calendar_name: "Primary".to_string(),
            html_link: None,
            meeting_url: None,
            status: "confirmed".to_string(),
        }))
        .expect("dto serializes");

        assert!(!raw.contains("calendarId"));
        assert!(!raw.contains("calendar_id"));
    }

    #[test]
    fn reminder_schedule_key_uses_event_id() {
        let key = reminder_schedule_key("evt-abc");
        assert_eq!(key, "reminder:evt-abc");
    }

    #[test]
    fn pending_reminder_delays_returns_future_events_only() {
        let now = "2026-03-26T10:00:00Z";
        let events = vec![
            // future — should schedule
            CalendarEvent {
                id: "a".to_string(),
                title: "Standup".to_string(),
                start_at: "2026-03-26T10:30:00Z".to_string(),
                end_at: "2026-03-26T11:00:00Z".to_string(),
                all_day: false,
                location: None,
                description: None,
                calendar_id: "primary".to_string(),
                calendar_name: "Primary".to_string(),
                html_link: None,
                meeting_url: None,
                status: "confirmed".to_string(),
            },
            // past — should not schedule
            CalendarEvent {
                id: "b".to_string(),
                title: "Old meeting".to_string(),
                start_at: "2026-03-26T09:00:00Z".to_string(),
                end_at: "2026-03-26T09:30:00Z".to_string(),
                all_day: false,
                location: None,
                description: None,
                calendar_id: "primary".to_string(),
                calendar_name: "Primary".to_string(),
                html_link: None,
                meeting_url: None,
                status: "confirmed".to_string(),
            },
            // all-day — should not schedule
            CalendarEvent {
                id: "c".to_string(),
                title: "Holiday".to_string(),
                start_at: "2026-03-26".to_string(),
                end_at: "2026-03-27".to_string(),
                all_day: true,
                location: None,
                description: None,
                calendar_id: "primary".to_string(),
                calendar_name: "Primary".to_string(),
                html_link: None,
                meeting_url: None,
                status: "confirmed".to_string(),
            },
        ];

        let delays = pending_reminder_delays(&events, now).expect("delays");

        assert_eq!(delays.len(), 1);
        assert_eq!(delays[0].0, "a");
        assert_eq!(delays[0].1, 30 * 60); // 30 minutes in seconds
    }

    #[test]
    fn upsert_task_link_deduplicates_links() {
        let mut state = StoredCalendarState::default();
        upsert_task_link(&mut state, "task-1", "evt-1", "linked");
        upsert_task_link(&mut state, "task-1", "evt-1", "created");

        assert_eq!(state.task_links.len(), 1);
        assert_eq!(state.task_links[0].task_id, "task-1");
        assert_eq!(state.task_links[0].event_id, "evt-1");
        assert_eq!(state.task_links[0].link_type, "created");
    }
}
