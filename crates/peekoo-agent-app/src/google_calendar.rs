use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarEvent {
    pub id: String,
    pub title: String,
    pub start_at: String,
    pub end_at: String,
    pub all_day: bool,
    pub location: Option<String>,
    pub calendar_name: String,
    pub html_link: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarEventBucket {
    pub id: String,
    pub title: String,
    pub start_at: String,
    pub end_at: String,
    pub all_day: bool,
    pub location: Option<String>,
    pub calendar_name: String,
    pub html_link: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BucketedCalendarEvents {
    pub upcoming: Vec<CalendarEventBucket>,
    pub today: Vec<CalendarEventBucket>,
    pub week: Vec<CalendarEventBucket>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderState {
    pub notified_event_ids: Vec<String>,
}

pub fn bucket_events(
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

    let upcoming = future_events
        .iter()
        .take(upcoming_limit)
        .map(|event| event.bucket.clone())
        .collect();

    let today_events = future_events
        .iter()
        .filter(|event| event.start.date_naive() == today || event.end.date_naive() == today)
        .map(|event| event.bucket.clone())
        .collect();

    let week = future_events
        .iter()
        .filter(|event| {
            let event_day = event.start.date_naive();
            event_day > today && event_day >= week_start && event_day < week_end
        })
        .map(|event| event.bucket.clone())
        .collect();

    Ok(BucketedCalendarEvents {
        upcoming,
        today: today_events,
        week,
    })
}

pub fn due_notification_ids(
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

pub fn reminder_id(event_id: &str, start_at: &str) -> String {
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
        let day = NaiveDate::parse_from_str(&event.start_at, "%Y-%m-%d")
            .map_err(|e| format!("Invalid all-day start '{}': {e}", event.start_at))?;
        return Ok(day
            .and_hms_opt(0, 0, 0)
            .expect("midnight is valid")
            .and_utc());
    }

    parse_datetime(&event.start_at)
}

fn parse_event_end(event: &CalendarEvent, today: NaiveDate) -> Result<DateTime<Utc>, String> {
    if event.all_day {
        let end_day = NaiveDate::parse_from_str(&event.end_at, "%Y-%m-%d")
            .map_err(|e| format!("Invalid all-day end '{}': {e}", event.end_at))?;
        return Ok(end_day
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
        .map_err(|e| format!("Invalid RFC3339 datetime '{value}': {e}"))
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
