use peekoo_scheduler::Scheduler;

#[tokio::test]
async fn fires_repeating_schedules_and_reports_remaining_time() {
    let scheduler = Scheduler::new();
    let fired = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let fired_clone = std::sync::Arc::clone(&fired);

    let handle = scheduler.start(move |owner, key| {
        fired_clone.lock().unwrap().push((owner, key));
    });

    scheduler
        .set("health-reminders", "water", 1, true)
        .expect("schedule should be accepted");

    tokio::time::sleep(std::time::Duration::from_millis(2200)).await;

    let entries = fired.lock().unwrap().clone();
    assert!(
        entries.len() >= 2,
        "expected at least two firings, got {entries:?}"
    );
    assert!(
        entries
            .iter()
            .all(|entry| entry == &("health-reminders".to_string(), "water".to_string()))
    );

    let listed = scheduler.list("health-reminders");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].key, "water");
    assert!(listed[0].time_remaining_secs <= 1);

    scheduler.shutdown_token().cancel();
    handle.join().expect("scheduler task should stop cleanly");
}

#[tokio::test]
async fn cancel_prevents_future_fires() {
    let scheduler = Scheduler::new();
    let fired = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let fired_clone = std::sync::Arc::clone(&fired);

    let handle = scheduler.start(move |owner, key| {
        fired_clone.lock().unwrap().push((owner, key));
    });

    scheduler
        .set("health-reminders", "eye-rest", 1, true)
        .expect("schedule should be accepted");
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    scheduler.cancel("health-reminders", "eye-rest");
    let after_first_fire = fired.lock().unwrap().len();

    tokio::time::sleep(std::time::Duration::from_millis(1300)).await;

    assert_eq!(fired.lock().unwrap().len(), after_first_fire);

    scheduler.shutdown_token().cancel();
    handle.join().expect("scheduler task should stop cleanly");
}

#[test]
fn start_does_not_require_caller_tokio_runtime() {
    let scheduler = Scheduler::new();
    let handle = scheduler.start(|_, _| {});

    std::thread::sleep(std::time::Duration::from_millis(50));

    scheduler.shutdown_token().cancel();
    handle.join().expect("scheduler thread should stop cleanly");
}
