use peekoo_core_domain::task::{Task, TaskPriority};
use peekoo_event_bus::{EventBus, EventEnvelope};

pub struct TaskUseCases {
    pub bus: EventBus,
}

impl TaskUseCases {
    pub fn new(bus: EventBus) -> Self {
        Self { bus }
    }

    pub fn create_task(&self, id: &str, title: &str, priority: TaskPriority) -> Task {
        let task = Task::new(id, title, priority);
        let _ = self.bus.publish(EventEnvelope {
            trace_id: format!("trace-{id}"),
            event_type: "v1.task.created".to_string(),
            schema_version: "v1".to_string(),
            payload: serde_json::json!({
                "id": task.id,
                "title": task.title,
            }),
        });
        task
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_task_emits_domain_event() {
        let bus = EventBus::new(8);
        let mut rx = bus.subscribe();
        let use_cases = TaskUseCases::new(bus);
        let task = use_cases.create_task("task-1", "Write tech spec", TaskPriority::High);
        assert_eq!(task.title, "Write tech spec");

        let emitted = rx.try_recv().expect("event emitted");
        assert_eq!(emitted.event_type, "v1.task.created");
    }
}
