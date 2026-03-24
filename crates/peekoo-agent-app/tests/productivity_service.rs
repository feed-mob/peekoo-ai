use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use peekoo_agent_app::productivity::ProductivityService;

fn create_test_service() -> ProductivityService {
    let conn = Arc::new(Mutex::new(
        Connection::open_in_memory().expect("Failed to create in-memory database"),
    ));
    ProductivityService::new(conn)
}

#[test]
fn create_task_rejects_empty_title() {
    let service = create_test_service();

    let result = service.create_task(
        "   ",
        "high",
        "user",
        &[],
        None,
        None,
        None,
        None,
        None,
        None,
    );

    assert_eq!(result.unwrap_err(), "Task title cannot be empty");
}

#[test]
fn create_task_rejects_unknown_priority() {
    let service = create_test_service();

    let result = service.create_task(
        "Write docs",
        "urgent",
        "user",
        &[],
        None,
        None,
        None,
        None,
        None,
        None,
    );

    assert_eq!(result.unwrap_err(), "Invalid task priority: urgent");
}

#[test]
fn pomodoro_lifecycle_transitions() {
    let service = create_test_service();
    let started = service.start_pomodoro(25).expect("start should succeed");
    assert_eq!(started.state, "running");

    let paused = service
        .pause_pomodoro(&started.id)
        .expect("pause should succeed");
    assert_eq!(paused.state, "paused");

    let resumed = service
        .resume_pomodoro(&started.id)
        .expect("resume should succeed");
    assert_eq!(resumed.state, "running");

    let finished = service
        .finish_pomodoro(&started.id)
        .expect("finish should succeed");
    assert_eq!(finished.state, "completed");
}
