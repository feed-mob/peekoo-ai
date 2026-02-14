use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub trace_id: String,
    pub event_type: String,
    pub schema_version: String,
    pub payload: Value,
}

#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<EventEnvelope>,
}

impl EventBus {
    pub fn new(buffer_size: usize) -> Self {
        let (tx, _) = broadcast::channel(buffer_size);
        Self { tx }
    }

    pub fn publish(&self, event: EventEnvelope) -> Result<usize, broadcast::error::SendError<EventEnvelope>> {
        self.tx.send(event)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<EventEnvelope> {
        self.tx.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn event_is_delivered_to_subscriber() {
        let bus = EventBus::new(8);
        let mut rx = bus.subscribe();
        let event = EventEnvelope {
            trace_id: "t-1".to_string(),
            event_type: "task.created".to_string(),
            schema_version: "v1".to_string(),
            payload: serde_json::json!({"id":"task-1"}),
        };
        let _ = bus.publish(event.clone());
        let got = rx.recv().await.expect("receive event");
        assert_eq!(got.event_type, event.event_type);
    }
}
