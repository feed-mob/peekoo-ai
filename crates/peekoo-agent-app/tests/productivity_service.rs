use std::sync::{Arc, Mutex};

use peekoo_agent_app::productivity::ProductivityService;
use rusqlite::Connection;

fn setup_service() -> ProductivityService {
    let conn = Connection::open_in_memory().expect("in-memory db");
    conn.execute_batch(
        r#"
        CREATE TABLE tasks (
          id TEXT PRIMARY KEY,
          title TEXT NOT NULL,
          notes TEXT,
          status TEXT NOT NULL,
          priority TEXT NOT NULL,
          due_at TEXT,
          source TEXT,
          assignee TEXT NOT NULL DEFAULT 'user',
          labels_json TEXT NOT NULL DEFAULT '[]',
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL
        );
        CREATE TABLE task_events (
          id TEXT PRIMARY KEY,
          task_id TEXT NOT NULL,
          event_type TEXT NOT NULL,
          payload_json TEXT NOT NULL,
          created_at TEXT NOT NULL
        );
        "#,
    )
    .expect("schema");
    ProductivityService::new(Arc::new(Mutex::new(conn)))
}

#[test]
fn create_task_rejects_empty_title() {
    let service = setup_service();
    let result = service.create_task("   ", "high", "user", &[]);
    assert_eq!(result.unwrap_err(), "Task title cannot be empty");
}

#[test]
fn create_task_rejects_unknown_priority() {
    let service = setup_service();
    let result = service.create_task("Write docs", "urgent", "user", &[]);
    assert_eq!(result.unwrap_err(), "Invalid task priority: urgent");
}

#[test]
fn create_task_persists_to_db() {
    let service = setup_service();
    let task = service
        .create_task("Write docs", "high", "user", &[])
        .expect("create should succeed");

    assert!(!task.id.is_empty());
    assert_eq!(task.title, "Write docs");
    assert_eq!(task.priority, "high");
    assert_eq!(task.status, "todo");
    assert_eq!(task.assignee, "user");
    assert!(task.labels.is_empty());

    // Verify it's in the list
    let tasks = service.list_tasks().expect("list should succeed");
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].id, task.id);
}

#[test]
fn create_task_with_labels_and_assignee() {
    let service = setup_service();
    let labels = vec!["bug".to_string(), "urgent".to_string()];
    let task = service
        .create_task("Fix login", "medium", "agent", &labels)
        .expect("create should succeed");

    assert_eq!(task.assignee, "agent");
    assert_eq!(task.labels, labels);
}

#[test]
fn toggle_task_changes_status() {
    let service = setup_service();
    let task = service
        .create_task("Write docs", "medium", "user", &[])
        .expect("create should succeed");

    // Toggle to done
    let toggled = service
        .toggle_task(&task.id)
        .expect("toggle should succeed");
    assert_eq!(toggled.status, "done");

    // Toggle back to todo
    let toggled_back = service
        .toggle_task(&task.id)
        .expect("toggle should succeed");
    assert_eq!(toggled_back.status, "todo");
}

#[test]
fn update_task_modifies_fields() {
    let service = setup_service();
    let task = service
        .create_task("Write docs", "medium", "user", &[])
        .expect("create should succeed");

    let updated = service
        .update_task(
            &task.id,
            Some("Write better docs"),
            Some("high"),
            Some("in_progress"),
            Some("agent"),
            Some(&["docs".to_string()]),
        )
        .expect("update should succeed");

    assert_eq!(updated.title, "Write better docs");
    assert_eq!(updated.priority, "high");
    assert_eq!(updated.status, "in_progress");
    assert_eq!(updated.assignee, "agent");
    assert_eq!(updated.labels, vec!["docs"]);
}

#[test]
fn delete_task_removes_from_db() {
    let service = setup_service();
    let task = service
        .create_task("Write docs", "medium", "user", &[])
        .expect("create should succeed");

    assert_eq!(service.list_tasks().unwrap().len(), 1);

    service
        .delete_task(&task.id)
        .expect("delete should succeed");
    assert_eq!(service.list_tasks().unwrap().len(), 0);
}

#[test]
fn crud_operations_write_events() {
    let service = setup_service();
    let task = service
        .create_task("Write docs", "medium", "user", &[])
        .expect("create should succeed");

    // Created event
    let events = service.list_task_events(10).expect("events should succeed");
    assert!(events.iter().any(|e| e.event_type == "created"));

    // Toggle writes event
    service
        .toggle_task(&task.id)
        .expect("toggle should succeed");
    let events = service.list_task_events(10).expect("events should succeed");
    assert!(events.iter().any(|e| e.event_type == "status_changed"));

    // Delete writes event
    service
        .delete_task(&task.id)
        .expect("delete should succeed");
    let events = service.list_task_events(10).expect("events should succeed");
    assert!(events.iter().any(|e| e.event_type == "deleted"));
}

#[test]
fn pomodoro_lifecycle_transitions() {
    let service = setup_service();
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
