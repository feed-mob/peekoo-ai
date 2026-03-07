use std::collections::VecDeque;
use std::sync::Mutex;

/// An event emitted by a plugin, to be forwarded to the host application.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PluginEvent {
    pub source_plugin: String,
    pub event: String,
    pub payload: serde_json::Value,
}

/// Deferred event queue.
///
/// Plugins emit events via the `peekoo_emit_event` host function during WASM
/// execution. Because the registry lock is held while a plugin runs, we cannot
/// immediately dispatch events to other plugins (that would be re-entrant).
/// Instead, events are enqueued and drained after each plugin call returns.
pub struct EventBus {
    outbound_queue: Mutex<VecDeque<PluginEvent>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            outbound_queue: Mutex::new(VecDeque::new()),
        }
    }

    /// Enqueue an event emitted by a plugin.
    pub fn enqueue(&self, event: PluginEvent) {
        match self.outbound_queue.lock() {
            Ok(mut queue) => queue.push_back(event),
            Err(e) => tracing::warn!("EventBus: mutex poisoned on enqueue: {e}"),
        }
    }

    /// Drain all queued events. Called after each plugin call returns.
    pub fn drain(&self) -> Vec<PluginEvent> {
        match self.outbound_queue.lock() {
            Ok(mut q) => q.drain(..).collect(),
            Err(e) => {
                tracing::warn!("EventBus: mutex poisoned on drain: {e}");
                Vec::new()
            }
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
