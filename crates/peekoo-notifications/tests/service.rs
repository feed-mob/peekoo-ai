use peekoo_notifications::{Notification, NotificationService};

#[test]
fn sends_notifications_when_dnd_is_disabled() {
    let (service, mut receiver) = NotificationService::new();

    let delivered = service.notify(Notification {
        source: "health-reminders".to_string(),
        title: "Drink water".to_string(),
        body: "Take a water break".to_string(),
    });

    assert!(delivered);

    let notification = receiver.try_recv().expect("notification should be queued");
    assert_eq!(notification.source, "health-reminders");
    assert_eq!(notification.title, "Drink water");
}

#[test]
fn suppresses_notifications_when_dnd_is_enabled() {
    let (service, mut receiver) = NotificationService::new();
    service.set_dnd(true);

    let delivered = service.notify(Notification {
        source: "health-reminders".to_string(),
        title: "Stand up".to_string(),
        body: "Time to stretch".to_string(),
    });

    assert!(!delivered);
    assert!(receiver.try_recv().is_err());
}
