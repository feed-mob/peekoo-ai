use std::sync::Mutex;

/// A mood reaction queued by a plugin for the sprite frontend.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MoodReaction {
    pub trigger: String,
    pub sticky: bool,
}

/// Thread-safe queue of sprite mood reactions.
///
/// Plugins push mood changes via the `peekoo_set_mood` host function.
/// The Tauri flush loop drains this queue and emits `pet:react` events
/// to the desktop pet frontend.
pub struct MoodReactionService {
    queue: Mutex<Vec<MoodReaction>>,
}

impl Default for MoodReactionService {
    fn default() -> Self {
        Self::new()
    }
}

impl MoodReactionService {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(Vec::new()),
        }
    }

    /// Queue a mood reaction.
    pub fn set(&self, trigger: &str, sticky: bool) {
        if let Ok(mut q) = self.queue.lock() {
            q.push(MoodReaction {
                trigger: trigger.to_string(),
                sticky,
            });
        }
    }

    /// Drain all queued mood reactions.
    pub fn drain(&self) -> Vec<MoodReaction> {
        match self.queue.lock() {
            Ok(mut q) => q.drain(..).collect(),
            Err(_) => Vec::new(),
        }
    }
}
