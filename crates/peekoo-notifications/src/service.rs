use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notification {
    pub source: String,
    pub title: String,
    pub body: String,
}

pub struct NotificationService {
    dnd_active: AtomicBool,
    sender: UnboundedSender<Notification>,
}

impl NotificationService {
    pub fn new() -> (Self, UnboundedReceiver<Notification>) {
        let (sender, receiver) = unbounded_channel();
        (
            Self {
                dnd_active: AtomicBool::new(false),
                sender,
            },
            receiver,
        )
    }

    pub fn notify(&self, notification: Notification) -> bool {
        if self.is_dnd() {
            tracing::debug!(
                source = notification.source,
                "Notification suppressed by DND"
            );
            return false;
        }

        self.sender.send(notification).is_ok()
    }

    pub fn set_dnd(&self, active: bool) {
        self.dnd_active.store(active, Ordering::Release);
    }

    pub fn is_dnd(&self) -> bool {
        self.dnd_active.load(Ordering::Acquire)
    }
}
