use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use peekoo_agent_app::SqliteTaskService;

fn create_test_service() -> SqliteTaskService {
    let conn = Arc::new(Mutex::new(
        Connection::open_in_memory().expect("Failed to create in-memory database"),
    ));
    SqliteTaskService::new(conn)
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
