use std::sync::{Arc, Mutex};

use peekoo_task_app::SqliteTaskService;

fn create_test_service() -> SqliteTaskService {
    let conn = Arc::new(Mutex::new(peekoo_persistence_sqlite::setup_test_db()));
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

#[test]
fn update_task_status_sets_finished_at_when_marked_done_and_clears_it_when_reopened() {
    let service = create_test_service();

    let task = service
        .create_task(
            "Write docs",
            "high",
            "user",
            &[],
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .expect("task created");

    let done = service
        .update_task_status(&task.id, peekoo_task_domain::TaskStatus::Done)
        .expect("mark done");
    assert_eq!(done.status, "done");
    assert!(done.finished_at.is_some());
    assert_eq!(
        done.updated_at,
        done.finished_at.clone().expect("finished_at set")
    );

    let reopened = service
        .update_task_status(&task.id, peekoo_task_domain::TaskStatus::InProgress)
        .expect("reopen task");
    assert_eq!(reopened.status, "in_progress");
    assert!(reopened.finished_at.is_none());
}

#[test]
fn migration_backfill_sets_finished_at_from_updated_at_for_existing_done_tasks() {
    let conn = rusqlite::Connection::open_in_memory().expect("Failed to create in-memory database");
    peekoo_persistence_sqlite::apply_migration(&conn, "0001_init");
    peekoo_persistence_sqlite::apply_migration(&conn, "0005_task_extensions");
    peekoo_persistence_sqlite::apply_migration(&conn, "0006_task_scheduling_and_recurrence");
    peekoo_persistence_sqlite::apply_migration(&conn, "0007_recurrence_time_of_day");
    peekoo_persistence_sqlite::apply_migration(&conn, "0009_agent_task_assignment");

    conn.execute(
        "INSERT INTO tasks (id, title, notes, status, priority, due_at, source, created_at, updated_at, assignee, labels_json) VALUES (?1, ?2, NULL, 'done', 'high', NULL, NULL, ?3, ?4, 'user', '[]')",
        rusqlite::params!["task-1", "Done task", "2026-03-20T08:00:00Z", "2026-03-24T12:00:00Z"],
    )
    .expect("seed task");

    peekoo_persistence_sqlite::apply_migration(&conn, "0011_task_finished_at");

    let finished_at: Option<String> = conn
        .query_row(
            "SELECT finished_at FROM tasks WHERE id = 'task-1'",
            [],
            |row| row.get(0),
        )
        .expect("load finished_at");

    assert_eq!(finished_at.as_deref(), Some("2026-03-24T12:00:00Z"));
}
