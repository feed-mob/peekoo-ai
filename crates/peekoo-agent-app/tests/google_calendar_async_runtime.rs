use std::sync::Arc;

use peekoo_agent_app::google_calendar_service::GoogleCalendarService;
use peekoo_notifications::NotificationService;

#[tokio::test]
async fn panel_snapshot_runs_inside_tokio_runtime_without_panicking() {
    let (notifications, _receiver) = NotificationService::new();
    let service = GoogleCalendarService::new(Arc::new(notifications)).expect("service");

    let snapshot = service.panel_snapshot(false).await.expect("snapshot");

    assert!(snapshot.upcoming.len() <= 5);
}
