use peekoo_agent_app::google_calendar::{
    bucket_events, due_notification_ids, CalendarEvent, CalendarEventBucket, ReminderState,
};

fn event(id: &str, title: &str, start_at: &str, end_at: &str) -> CalendarEvent {
    CalendarEvent {
        id: id.to_string(),
        title: title.to_string(),
        start_at: start_at.to_string(),
        end_at: end_at.to_string(),
        all_day: false,
        location: None,
        calendar_name: "Primary".to_string(),
        html_link: None,
        status: "confirmed".to_string(),
    }
}

#[test]
fn buckets_events_into_upcoming_today_and_week_views() {
    let now = "2026-03-19T09:00:00Z";
    let events = vec![
        event(
            "soon-1",
            "Standup",
            "2026-03-19T09:30:00Z",
            "2026-03-19T10:00:00Z",
        ),
        event(
            "soon-2",
            "Planning",
            "2026-03-19T11:00:00Z",
            "2026-03-19T12:00:00Z",
        ),
        event(
            "today-late",
            "1:1",
            "2026-03-19T18:00:00Z",
            "2026-03-19T18:30:00Z",
        ),
        event(
            "week-1",
            "Demo",
            "2026-03-21T15:00:00Z",
            "2026-03-21T16:00:00Z",
        ),
        event(
            "week-2",
            "Retro",
            "2026-03-22T14:00:00Z",
            "2026-03-22T15:00:00Z",
        ),
        event(
            "next-week",
            "Roadmap",
            "2026-03-24T09:00:00Z",
            "2026-03-24T10:00:00Z",
        ),
    ];

    let bucketed = bucket_events(&events, now, 5).expect("bucket events");

    assert_eq!(
        ids(&bucketed.upcoming),
        vec!["soon-1", "soon-2", "today-late", "week-1", "week-2"]
    );
    assert_eq!(ids(&bucketed.today), vec!["soon-1", "soon-2", "today-late"]);
    assert_eq!(ids(&bucketed.week), vec!["week-1", "week-2"]);
}

#[test]
fn upcoming_view_is_capped_to_requested_size() {
    let now = "2026-03-19T09:00:00Z";
    let events = vec![
        event("1", "One", "2026-03-19T09:10:00Z", "2026-03-19T09:20:00Z"),
        event("2", "Two", "2026-03-19T09:30:00Z", "2026-03-19T09:40:00Z"),
        event("3", "Three", "2026-03-19T09:50:00Z", "2026-03-19T10:00:00Z"),
    ];

    let bucketed = bucket_events(&events, now, 2).expect("bucket events");

    assert_eq!(ids(&bucketed.upcoming), vec!["1", "2"]);
}

#[test]
fn reminder_logic_skips_all_day_and_already_notified_events() {
    let now = "2026-03-19T09:00:00Z";
    let mut all_day = event("all-day", "OOO", "2026-03-19", "2026-03-20");
    all_day.all_day = true;

    let events = vec![
        event(
            "fresh",
            "Design review",
            "2026-03-19T09:08:00Z",
            "2026-03-19T09:38:00Z",
        ),
        event(
            "later",
            "Lunch",
            "2026-03-19T10:00:00Z",
            "2026-03-19T11:00:00Z",
        ),
        all_day,
    ];
    let reminder_state = ReminderState {
        notified_event_ids: vec!["later@2026-03-19T10:00:00Z".to_string()],
    };

    let due = due_notification_ids(&events, now, 10, &reminder_state).expect("due reminders");

    assert_eq!(due, vec!["fresh@2026-03-19T09:08:00Z"]);
}

fn ids(events: &[CalendarEventBucket]) -> Vec<&str> {
    events.iter().map(|event| event.id.as_str()).collect()
}
