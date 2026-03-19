#![cfg_attr(not(test), no_main)]

use chrono::{DateTime, Datelike, Days, Duration, NaiveDate, SecondsFormat, Utc, Weekday};
use peekoo_plugin_sdk::prelude::*;

const CLIENT_ID_KEY: &str = "client-id";
const CLIENT_SECRET_KEY: &str = "client-secret";
const TOKEN_BUNDLE_KEY: &str = "token-bundle";
const CONNECTED_ACCOUNT_KEY: &str = "connected-account";
const STATE_KEY: &str = "calendar-state";
const SYNC_SCHEDULE_KEY: &str = "calendar-sync";
const GOOGLE_PROVIDER_ID: &str = "google-calendar";
const DEFAULT_REFRESH_INTERVAL_SECS: i64 = 300;
const DEFAULT_REMINDER_LEAD_MINUTES: i64 = 10;
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
    oauth_flow_id: Option<String>,
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
    upcoming: Vec<CalendarEventBucket>,
    today: Vec<CalendarEventBucket>,
    week: Vec<CalendarEventBucket>,
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
    calendar_name: String,
    html_link: Option<String>,
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
    calendar_name: String,
    html_link: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BucketedCalendarEvents {
    upcoming: Vec<CalendarEventBucket>,
    today: Vec<CalendarEventBucket>,
    week: Vec<CalendarEventBucket>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReminderState {
    notified_event_ids: Vec<String>,
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
struct GoogleEventItem {
    id: String,
    summary: Option<String>,
    status: Option<String>,
    html_link: Option<String>,
    location: Option<String>,
    start: GoogleEventDateTime,
    end: GoogleEventDateTime,
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
    if event["event"].as_str() == Some("schedule:fired")
        && event["payload"]["key"].as_str() == Some(SYNC_SCHEDULE_KEY)
    {
        let _ = refresh_snapshot(false);
    }
    Ok(r#"{"ok":true}"#.to_string())
}

#[plugin_fn]
pub fn tool_google_calendar_set_client_json(input: String) -> FnResult<String> {
    let payload: Value = serde_json::from_str(&input)?;
    let client_json = payload["clientJson"]
        .as_str()
        .ok_or_else(|| Error::msg("Missing clientJson"))?;
    let credentials = parse_google_client_json(client_json).map_err(Error::msg)?;
    peekoo::secrets::set(CLIENT_ID_KEY, &credentials.client_id)?;
    peekoo::secrets::set(CLIENT_SECRET_KEY, &credentials.client_secret)?;
    Ok(r#"{"ok":true}"#.to_string())
}

#[plugin_fn]
pub fn tool_google_calendar_connect_start(_: String) -> FnResult<String> {
    let credentials = load_client_credentials().map_err(Error::msg)?;
    let result = peekoo::oauth::start(peekoo::oauth::StartRequest {
        provider_id: GOOGLE_PROVIDER_ID,
        client_id: &credentials.client_id,
        client_secret: Some(&credentials.client_secret),
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
        upcoming: bucketed.upcoming,
        today: bucketed.today,
        week: bucketed.week,
    })
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
    match fetch_events(&bundle.access_token) {
        Ok(events) => {
            state.cached_events = events;
            state.last_sync_at = Some(Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true));
            state.last_error = None;
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

fn fetch_events(access_token: &str) -> Result<Vec<CalendarEvent>, String> {
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
    let url = format!(
        "https://www.googleapis.com/calendar/v3/calendars/primary/events?singleEvents=true&orderBy=startTime&timeMin={}&timeMax={}",
        percent_encode_component(&time_min.to_rfc3339_opts(SecondsFormat::Secs, true)),
        percent_encode_component(&time_max.to_rfc3339_opts(SecondsFormat::Secs, true)),
    );
    let response = peekoo::http::request(peekoo::http::Request {
        method: "GET",
        url: &url,
        headers: vec![("Authorization", &format!("Bearer {access_token}"))],
        body: None,
    })
    .map_err(|e| e.to_string())?;
    if response.status >= 400 {
        return Err(format!(
            "Google Calendar fetch failed ({}): {}",
            response.status, response.body
        ));
    }
    let parsed: GoogleEventsResponse =
        serde_json::from_str(&response.body).map_err(|e| e.to_string())?;
    parsed.items.into_iter().map(normalize_event).collect()
}

fn fetch_account_profile(access_token: &str) -> Result<GoogleAccountProfile, String> {
    let response = peekoo::http::request(peekoo::http::Request {
        method: "GET",
        url: "https://www.googleapis.com/oauth2/v2/userinfo",
        headers: vec![("Authorization", &format!("Bearer {access_token}"))],
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
    let due_ids = due_notification_ids(
        &state.cached_events,
        &Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        DEFAULT_REMINDER_LEAD_MINUTES,
        &ReminderState {
            notified_event_ids: state.notified_event_ids.clone(),
        },
    )?;

    for notification_id in due_ids {
        if let Some(event) = state
            .cached_events
            .iter()
            .find(|event| reminder_id(&event.id, &event.start_at) == notification_id)
        {
            let when = if event.all_day {
                "today".to_string()
            } else {
                event.start_at.clone()
            };
            let _ = peekoo::notify::send(&event.title, &format!("Starts at {when}"));
            state.notified_event_ids.push(notification_id);
        }
    }

    prune_notified_ids(state);
    Ok(())
}

fn prune_notified_ids(state: &mut StoredCalendarState) {
    let active_ids = state
        .cached_events
        .iter()
        .map(|event| reminder_id(&event.id, &event.start_at))
        .collect::<std::collections::HashSet<_>>();
    state
        .notified_event_ids
        .retain(|notification_id| active_ids.contains(notification_id));
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

fn normalize_event(event: GoogleEventItem) -> Result<CalendarEvent, String> {
    let (start_at, all_day) = normalize_event_time(&event.start)?;
    let (end_at, _) = normalize_event_time(&event.end)?;
    Ok(CalendarEvent {
        id: event.id,
        title: event
            .summary
            .unwrap_or_else(|| "Untitled event".to_string()),
        start_at,
        end_at,
        all_day,
        location: event.location,
        calendar_name: "Primary".to_string(),
        html_link: event.html_link,
        status: event.status.unwrap_or_else(|| "confirmed".to_string()),
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

fn due_notification_ids(
    events: &[CalendarEvent],
    now_iso: &str,
    reminder_lead_minutes: i64,
    reminder_state: &ReminderState,
) -> Result<Vec<String>, String> {
    let now = parse_datetime(now_iso)?;
    let lead = Duration::minutes(reminder_lead_minutes);
    let mut due = Vec::new();
    for event in events {
        if event.all_day {
            continue;
        }
        let start = parse_datetime(&event.start_at)?;
        if start < now || start > now + lead {
            continue;
        }
        let reminder_id = reminder_id(&event.id, &event.start_at);
        if reminder_state
            .notified_event_ids
            .iter()
            .any(|id| id == &reminder_id)
        {
            continue;
        }
        due.push(reminder_id);
    }
    due.sort();
    Ok(due)
}

fn reminder_id(event_id: &str, start_at: &str) -> String {
    format!("{event_id}@{start_at}")
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
            calendar_name: event.calendar_name.clone(),
            html_link: event.html_link.clone(),
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
                calendar_name: "Primary".to_string(),
                html_link: None,
                status: "confirmed".to_string(),
            },
            CalendarEvent {
                id: "b".to_string(),
                title: "Planning".to_string(),
                start_at: "2026-03-20T14:00:00Z".to_string(),
                end_at: "2026-03-20T15:00:00Z".to_string(),
                all_day: false,
                location: None,
                calendar_name: "Primary".to_string(),
                html_link: None,
                status: "confirmed".to_string(),
            },
        ];

        let bucketed = bucket_events(&events, "2026-03-19T09:00:00Z", 5).expect("bucket events");

        assert_eq!(bucketed.upcoming.len(), 2);
        assert_eq!(bucketed.today.len(), 1);
        assert_eq!(bucketed.week.len(), 1);
    }

    #[test]
    fn due_notification_ids_skip_already_notified_events() {
        let events = vec![CalendarEvent {
            id: "a".to_string(),
            title: "Daily standup".to_string(),
            start_at: "2026-03-19T10:05:00Z".to_string(),
            end_at: "2026-03-19T10:30:00Z".to_string(),
            all_day: false,
            location: None,
            calendar_name: "Primary".to_string(),
            html_link: None,
            status: "confirmed".to_string(),
        }];

        let ids = due_notification_ids(
            &events,
            "2026-03-19T10:00:00Z",
            10,
            &ReminderState {
                notified_event_ids: vec![reminder_id("a", "2026-03-19T10:05:00Z")],
            },
        )
        .expect("notification ids");

        assert!(ids.is_empty());
    }
}
