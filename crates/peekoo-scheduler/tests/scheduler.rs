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
        .set("health-reminders", "water", 1, true, None)
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
        .set("health-reminders", "eye-rest", 1, true, None)
        .expect("schedule should be accepted");
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    scheduler.cancel("health-reminders", "eye-rest");
    let after_first_fire = fired.lock().unwrap().len();

    tokio::time::sleep(std::time::Duration::from_millis(1300)).await;

    assert_eq!(fired.lock().unwrap().len(), after_first_fire);

    scheduler.shutdown_token().cancel();
    handle.join().expect("scheduler task should stop cleanly");
}

#[tokio::test]
async fn delay_secs_overrides_initial_fire_time() {
    let scheduler = Scheduler::new();
    let fired = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let fired_clone = std::sync::Arc::clone(&fired);

    let handle = scheduler.start(move |owner, key| {
        fired_clone.lock().unwrap().push((owner, key));
    });

    // Set a 10s interval but with a 1s initial delay
    scheduler
        .set("test", "fast-start", 10, true, Some(1))
        .expect("schedule should be accepted");

    // Should fire after ~1s, not ~10s
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
    let entries = fired.lock().unwrap().clone();
    assert_eq!(
        entries.len(),
        1,
        "expected exactly one firing after 1.5s with 1s delay, got {entries:?}"
    );

    // After the first fire, the repeat interval (10s) should be used.
    // Verify that it does NOT fire again within 2s (would fire at ~11s).
    tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
    assert_eq!(
        fired.lock().unwrap().len(),
        1,
        "should not have fired again within 2s of the first fire"
    );

    scheduler.shutdown_token().cancel();
    handle.join().expect("scheduler task should stop cleanly");
}

#[tokio::test]
async fn delay_secs_none_uses_full_interval() {
    let scheduler = Scheduler::new();
    let fired = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let fired_clone = std::sync::Arc::clone(&fired);

    let handle = scheduler.start(move |owner, key| {
        fired_clone.lock().unwrap().push((owner, key));
    });

    // delay_secs: None should behave like the original -- first fire after full interval
    scheduler
        .set("test", "normal", 1, true, None)
        .expect("schedule should be accepted");

    // Should not fire before ~1s
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    assert_eq!(
        fired.lock().unwrap().len(),
        0,
        "should not fire before the full 1s interval"
    );

    tokio::time::sleep(std::time::Duration::from_millis(700)).await;
    assert_eq!(
        fired.lock().unwrap().len(),
        1,
        "should fire once after ~1.2s"
    );

    scheduler.shutdown_token().cancel();
    handle.join().expect("scheduler task should stop cleanly");
}

#[tokio::test]
async fn delay_secs_zero_fires_immediately() {
    let scheduler = Scheduler::new();
    let fired = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let fired_clone = std::sync::Arc::clone(&fired);

    let handle = scheduler.start(move |owner, key| {
        fired_clone.lock().unwrap().push((owner, key));
    });

    // delay_secs: Some(0) should fire as soon as possible
    scheduler
        .set("test", "immediate", 5, true, Some(0))
        .expect("schedule should be accepted");

    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert!(
        !fired.lock().unwrap().is_empty(),
        "should fire almost immediately with delay_secs=0"
    );

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
