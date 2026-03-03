use peekoo_agent_app::productivity::ProductivityService;

#[test]
fn create_task_rejects_empty_title() {
    let service = ProductivityService::new();

    let result = service.create_task("   ", "high");

    assert_eq!(result.unwrap_err(), "Task title cannot be empty");
}

#[test]
fn create_task_rejects_unknown_priority() {
    let service = ProductivityService::new();

    let result = service.create_task("Write docs", "urgent");

    assert_eq!(result.unwrap_err(), "Invalid task priority: urgent");
}

#[test]
fn pomodoro_lifecycle_transitions() {
    let service = ProductivityService::new();
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
