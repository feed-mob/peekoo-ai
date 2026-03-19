use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use chrono::{Days, SecondsFormat, Utc};
use peekoo_agent_auth::provider::google_calendar;
use peekoo_agent_auth::{OAuthFlowStatus, OAuthService};
use peekoo_notifications::{Notification, NotificationService};
use peekoo_security::{
    FallbackSecretStore, FileSecretStore, KeyringSecretStore, SecretStore, SecretStoreError,
};
use serde::{Deserialize, Serialize};

use crate::google_calendar::{
    CalendarEvent, CalendarEventBucket, ReminderState, bucket_events, due_notification_ids,
    reminder_id,
};
use crate::settings::{OauthStartResponse, OauthStatusRequest};

const GOOGLE_CALENDAR_PROVIDER_ID: &str = "google-calendar";
const GOOGLE_CALENDAR_TOKEN_KEY: &str = "peekoo/google-calendar/oauth";
const GOOGLE_CALENDAR_CLIENT_ID_KEY: &str = "peekoo/google-calendar/client-id";
const GOOGLE_CALENDAR_CLIENT_SECRET_KEY: &str = "peekoo/google-calendar/client-secret";
const DEFAULT_REMINDER_LEAD_MINUTES: i64 = 10;
const DEFAULT_REFRESH_INTERVAL_SECS: i64 = 300;
const DEFAULT_UPCOMING_LIMIT: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoogleClientCredentials {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleAccountProfile {
    pub email: String,
    pub name: Option<String>,
    pub picture: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct GoogleClientJson {
    installed: Option<GoogleInstalledClient>,
    web: Option<GoogleWebClient>,
}

#[derive(Debug, Clone, Deserialize)]
struct GoogleInstalledClient {
    client_id: String,
    client_secret: String,
}

#[derive(Debug, Clone, Deserialize)]
struct GoogleWebClient {
    client_id: String,
    client_secret: String,
}

#[derive(Debug, Clone, Deserialize)]
struct GoogleAccountProfilePayload {
    email: Option<String>,
    name: Option<String>,
    picture: Option<String>,
}

pub fn parse_google_client_json(raw: &str) -> Result<GoogleClientCredentials, String> {
    let parsed: GoogleClientJson =
        serde_json::from_str(raw).map_err(|e| format!("Parse Google client json error: {e}"))?;

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

pub fn parse_google_account_profile(raw: &str) -> Result<GoogleAccountProfile, String> {
    let payload: GoogleAccountProfilePayload = serde_json::from_str(raw)
        .map_err(|e| format!("Parse Google account profile error: {e}"))?;

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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleCalendarOauthStatusDto {
    pub status: String,
    pub connected: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleCalendarStatusDto {
    pub connected: bool,
    pub client_configured: bool,
    pub client_json_uploaded: bool,
    pub effective_client_id: String,
    pub connected_account: Option<GoogleAccountProfile>,
    pub last_sync_at: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleCalendarPanelDto {
    pub status: GoogleCalendarStatusDto,
    pub upcoming: Vec<CalendarEventBucket>,
    pub today: Vec<CalendarEventBucket>,
    pub week: Vec<CalendarEventBucket>,
}

pub struct GoogleCalendarService {
    oauth: OAuthService,
    core: Arc<GoogleCalendarCore>,
    background_started: AtomicBool,
}

struct GoogleCalendarCore {
    secret_store: Arc<dyn SecretStore>,
    state_path: PathBuf,
    notifications: Arc<NotificationService>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredTokenBundle {
    access_token: String,
    refresh_token: Option<String>,
    expires_at_epoch: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct StoredCalendarState {
    connected_account: Option<GoogleAccountProfile>,
    last_sync_at: Option<String>,
    last_error: Option<String>,
    cached_events: Vec<CalendarEvent>,
    notified_event_ids: Vec<String>,
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

impl GoogleCalendarService {
    pub fn new(notifications: Arc<NotificationService>) -> Result<Self, String> {
        let fallback_root = peekoo_paths::peekoo_global_data_dir()?.join("secrets");
        let secret_store: Arc<dyn SecretStore> = Arc::new(FallbackSecretStore::new(
            Box::new(KeyringSecretStore::new("peekoo-desktop")),
            Box::new(FileSecretStore::new(fallback_root)),
        ));
        let state_dir = peekoo_paths::peekoo_global_data_dir()?.join("google-calendar");
        std::fs::create_dir_all(&state_dir)
            .map_err(|e| format!("Create Google Calendar state dir error: {e}"))?;

        Ok(Self {
            oauth: OAuthService::new(),
            core: Arc::new(GoogleCalendarCore {
                secret_store,
                state_path: state_dir.join("state.json"),
                notifications,
            }),
            background_started: AtomicBool::new(false),
        })
    }

    pub fn start_runtime(&self) {
        if self.background_started.swap(true, Ordering::AcqRel) {
            return;
        }

        let core = Arc::clone(&self.core);
        std::thread::spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(err) => {
                    tracing::warn!("Google Calendar runtime build error: {err}");
                    return;
                }
            };

            loop {
                if let Err(err) = runtime.block_on(core.tick()) {
                    tracing::warn!("Google Calendar tick error: {err}");
                }
                std::thread::sleep(Duration::from_secs(60));
            }
        });
    }

    pub fn connect_start(&self) -> Result<OauthStartResponse, String> {
        let credentials = self
            .core
            .load_client_credentials()?
            .ok_or_else(|| "Upload your Google OAuth client.json file first.".to_string())?;
        let started = self
            .oauth
            .start_google_calendar_with_secret(
                &credentials.client_id,
                Some(&credentials.client_secret),
            )
            .map_err(|e| format!("Google Calendar OAuth start error: {e}"))?;

        Ok(OauthStartResponse {
            flow_id: started.flow_id,
            authorize_url: started.authorize_url,
            opened_browser: false,
        })
    }

    pub async fn connect_status(
        &self,
        req: OauthStatusRequest,
    ) -> Result<GoogleCalendarOauthStatusDto, String> {
        let status = self
            .oauth
            .status(&req.flow_id)
            .await
            .map_err(|e| format!("Google Calendar OAuth status error: {e}"))?;

        if let Some(access_token) = status.access_token {
            let bundle = StoredTokenBundle {
                access_token,
                refresh_token: status.refresh_token,
                expires_at_epoch: status
                    .expires_at
                    .as_deref()
                    .and_then(|value| value.parse::<i64>().ok()),
            };
            self.core.save_token_bundle(&bundle)?;
            let connected_account = fetch_account_profile(&bundle.access_token).await?;
            self.core.save_connected_account(Some(connected_account))?;
            self.core.refresh_events(true).await?;
            return Ok(GoogleCalendarOauthStatusDto {
                status: OAuthFlowStatus::Completed.as_str().to_string(),
                connected: true,
                error: None,
            });
        }

        Ok(GoogleCalendarOauthStatusDto {
            status: status.status.as_str().to_string(),
            connected: self.core.load_token_bundle()?.is_some(),
            error: status.error,
        })
    }

    pub fn disconnect(&self) -> Result<(), String> {
        let _ = self.core.secret_store.delete(GOOGLE_CALENDAR_TOKEN_KEY);
        let mut state = self.core.load_state()?;
        state.connected_account = None;
        state.last_sync_at = None;
        state.last_error = None;
        state.cached_events.clear();
        state.notified_event_ids.clear();
        self.core.save_state(&state)
    }

    pub fn set_client_json(&self, client_json: &str) -> Result<(), String> {
        let credentials = parse_google_client_json(client_json)?;
        self.core.save_client_credentials(&credentials)
    }

    pub async fn panel_snapshot(&self, refresh: bool) -> Result<GoogleCalendarPanelDto, String> {
        if refresh {
            self.core.refresh_events(true).await?;
        }

        let state = self.core.load_state()?;
        let client_credentials = self.core.load_client_credentials()?;
        let now_iso = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
        let bucketed = bucket_events(&state.cached_events, &now_iso, DEFAULT_UPCOMING_LIMIT)?;

        Ok(GoogleCalendarPanelDto {
            status: GoogleCalendarStatusDto {
                connected: self.core.load_token_bundle()?.is_some(),
                client_configured: client_credentials.is_some(),
                client_json_uploaded: client_credentials.is_some(),
                effective_client_id: client_credentials
                    .as_ref()
                    .map(|credentials| credentials.client_id.clone())
                    .unwrap_or_default(),
                connected_account: state.connected_account.clone(),
                last_sync_at: state.last_sync_at,
                last_error: state.last_error,
            },
            upcoming: bucketed.upcoming,
            today: bucketed.today,
            week: bucketed.week,
        })
    }
}

impl GoogleCalendarCore {
    fn save_connected_account(
        &self,
        connected_account: Option<GoogleAccountProfile>,
    ) -> Result<(), String> {
        let mut state = self.load_state()?;
        state.connected_account = connected_account;
        self.save_state(&state)
    }

    fn load_client_credentials(&self) -> Result<Option<GoogleClientCredentials>, String> {
        let client_id = match self.secret_store.get(GOOGLE_CALENDAR_CLIENT_ID_KEY) {
            Ok(value) => value,
            Err(SecretStoreError::NotFound) => return Ok(None),
            Err(err) => return Err(format!("Read Google Calendar client id error: {err}")),
        };

        let client_secret = match self.secret_store.get(GOOGLE_CALENDAR_CLIENT_SECRET_KEY) {
            Ok(value) => value,
            Err(SecretStoreError::NotFound) => {
                return Err("Google Calendar client secret is missing. Upload client.json again.".to_string())
            }
            Err(err) => {
                return Err(format!("Read Google Calendar client secret error: {err}"))
            }
        };

        Ok(Some(GoogleClientCredentials {
            client_id,
            client_secret,
        }))
    }

    fn save_client_credentials(&self, credentials: &GoogleClientCredentials) -> Result<(), String> {
        self.secret_store
            .put(GOOGLE_CALENDAR_CLIENT_ID_KEY, &credentials.client_id)
            .map_err(|e| format!("Save Google Calendar client id error: {e}"))?;
        self.secret_store
            .put(GOOGLE_CALENDAR_CLIENT_SECRET_KEY, &credentials.client_secret)
            .map_err(|e| format!("Save Google Calendar client secret error: {e}"))
    }

    async fn tick(&self) -> Result<(), String> {
        let _ = self.refresh_events(false).await;
        self.notify_due_events()
    }

    fn notify_due_events(&self) -> Result<(), String> {
        let mut state = self.load_state()?;
        if state.cached_events.is_empty() {
            return Ok(());
        }

        let now_iso = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
        let due_ids = due_notification_ids(
            &state.cached_events,
            &now_iso,
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
                let _ = self.notifications.notify(Notification {
                    source: GOOGLE_CALENDAR_PROVIDER_ID.to_string(),
                    title: event.title.clone(),
                    body: format!("Starts at {when}"),
                });
                state.notified_event_ids.push(notification_id);
            }
        }

        self.prune_notified_ids(&mut state);
        self.save_state(&state)
    }

    async fn refresh_events(&self, force: bool) -> Result<(), String> {
        let Some(mut bundle) = self.load_token_bundle()? else {
            return Ok(());
        };

        if !force {
            let state = self.load_state()?;
            if let Some(last_sync_at) = state.last_sync_at.as_deref()
                && let Ok(last_sync) = chrono::DateTime::parse_from_rfc3339(last_sync_at)
                && (Utc::now() - last_sync.with_timezone(&Utc)).num_seconds()
                    < DEFAULT_REFRESH_INTERVAL_SECS
            {
                return Ok(());
            }
        }

        if token_expired_soon(&bundle) {
            let credentials = self
                .load_client_credentials()?
                .ok_or_else(|| "Upload your Google OAuth client.json file first.".to_string())?;
            bundle = self.refresh_token_bundle(
                &bundle,
                &credentials.client_id,
                Some(credentials.client_secret.as_str()),
            )
            .await?;
            self.save_token_bundle(&bundle)?;
        }

        match fetch_events(&bundle.access_token).await {
            Ok(events) => {
                let mut state = self.load_state()?;
                state.cached_events = events;
                state.last_sync_at = Some(Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true));
                state.last_error = None;
                self.prune_notified_ids(&mut state);
                self.save_state(&state)
            }
            Err(err) => {
                let mut state = self.load_state()?;
                state.last_error = Some(err);
                self.save_state(&state)
            }
        }
    }

    async fn refresh_token_bundle(
        &self,
        bundle: &StoredTokenBundle,
        client_id: &str,
        client_secret: Option<&str>,
    ) -> Result<StoredTokenBundle, String> {
        let refresh_token = bundle
            .refresh_token
            .as_deref()
            .ok_or_else(|| "Google Calendar refresh token is missing. Reconnect the account.".to_string())?;
        let refreshed = google_calendar::refresh_access_token(client_id, client_secret, refresh_token)
            .await
            .map_err(|e| format!("Google Calendar token refresh error: {e}"))?;

        Ok(StoredTokenBundle {
            access_token: refreshed.access_token,
            refresh_token: refreshed
                .refresh_token
                .or_else(|| bundle.refresh_token.clone()),
            expires_at_epoch: Some(Utc::now().timestamp() + refreshed.expires_in),
        })
    }

    fn load_state(&self) -> Result<StoredCalendarState, String> {
        if !self.state_path.exists() {
            return Ok(StoredCalendarState::default());
        }

        let raw = std::fs::read_to_string(&self.state_path)
            .map_err(|e| format!("Read Google Calendar state error: {e}"))?;
        serde_json::from_str(&raw).map_err(|e| format!("Parse Google Calendar state error: {e}"))
    }

    fn save_state(&self, state: &StoredCalendarState) -> Result<(), String> {
        let raw = serde_json::to_string_pretty(state)
            .map_err(|e| format!("Serialize Google Calendar state error: {e}"))?;
        std::fs::write(&self.state_path, raw)
            .map_err(|e| format!("Write Google Calendar state error: {e}"))
    }

    fn load_token_bundle(&self) -> Result<Option<StoredTokenBundle>, String> {
        match self.secret_store.get(GOOGLE_CALENDAR_TOKEN_KEY) {
            Ok(raw) => serde_json::from_str(&raw)
                .map(Some)
                .map_err(|e| format!("Parse Google Calendar token bundle error: {e}")),
            Err(SecretStoreError::NotFound) => Ok(None),
            Err(err) => Err(format!("Read Google Calendar token bundle error: {err}")),
        }
    }

    fn save_token_bundle(&self, bundle: &StoredTokenBundle) -> Result<(), String> {
        let raw = serde_json::to_string(bundle)
            .map_err(|e| format!("Serialize Google Calendar token bundle error: {e}"))?;
        self.secret_store
            .put(GOOGLE_CALENDAR_TOKEN_KEY, &raw)
            .map_err(|e| format!("Save Google Calendar token bundle error: {e}"))
    }

    fn prune_notified_ids(&self, state: &mut StoredCalendarState) {
        let active_ids = state
            .cached_events
            .iter()
            .map(|event| reminder_id(&event.id, &event.start_at))
            .collect::<std::collections::HashSet<_>>();
        state
            .notified_event_ids
            .retain(|notification_id| active_ids.contains(notification_id));
    }
}

fn token_expired_soon(bundle: &StoredTokenBundle) -> bool {
    bundle
        .expires_at_epoch
        .map(|expires_at| expires_at <= Utc::now().timestamp() + 60)
        .unwrap_or(false)
}

async fn fetch_events(access_token: &str) -> Result<Vec<CalendarEvent>, String> {
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

    let response = reqwest::Client::new()
        .get("https://www.googleapis.com/calendar/v3/calendars/primary/events")
        .query(&[
            ("singleEvents", "true"),
            ("orderBy", "startTime"),
            ("timeMin", &time_min.to_rfc3339_opts(SecondsFormat::Secs, true)),
            ("timeMax", &time_max.to_rfc3339_opts(SecondsFormat::Secs, true)),
        ])
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| format!("Google Calendar fetch error: {e}"))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format!("Read Google Calendar response error: {e}"))?;
    if !status.is_success() {
        return Err(format!("Google Calendar fetch failed ({status}): {body}"));
    }

    let parsed: GoogleEventsResponse =
        serde_json::from_str(&body).map_err(|e| format!("Parse Google Calendar response error: {e}"))?;

    Ok(parsed
        .items
        .into_iter()
        .map(normalize_event)
        .collect::<Result<Vec<_>, _>>()?)
}

async fn fetch_account_profile(access_token: &str) -> Result<GoogleAccountProfile, String> {
    let response = reqwest::Client::new()
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| format!("Google account profile fetch error: {e}"))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format!("Read Google account profile response error: {e}"))?;
    if !status.is_success() {
        return Err(format!(
            "Google account profile fetch failed ({status}): {body}"
        ));
    }

    parse_google_account_profile(&body)
}

fn normalize_event(event: GoogleEventItem) -> Result<CalendarEvent, String> {
    let (start_at, all_day) = normalize_event_time(&event.start)?;
    let (end_at, _) = normalize_event_time(&event.end)?;
    Ok(CalendarEvent {
        id: event.id,
        title: event.summary.unwrap_or_else(|| "Untitled event".to_string()),
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
