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
    inner: Mutex<MoodReactionState>,
}

struct MoodReactionState {
    queue: Vec<MoodReaction>,
    ui_ready: bool,
}

impl Default for MoodReactionService {
    fn default() -> Self {
        Self::new()
    }
}

impl MoodReactionService {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(MoodReactionState {
                queue: Vec::new(),
                ui_ready: false,
            }),
        }
    }

    /// Queue a mood reaction.
    pub fn set(&self, trigger: &str, sticky: bool) {
        if let Ok(mut state) = self.inner.lock() {
            state.queue.push(MoodReaction {
                trigger: trigger.to_string(),
                sticky,
            });
        }
    }

    /// Signal that the UI has mounted and is listening for mood events.
    pub fn mark_ui_ready(&self) {
        if let Ok(mut state) = self.inner.lock() {
            state.ui_ready = true;
        }
    }

    /// Drain all queued mood reactions.
    pub fn drain(&self) -> Vec<MoodReaction> {
        match self.inner.lock() {
            Ok(mut state) => {
                if !state.ui_ready {
                    return Vec::new();
                }
                state.queue.drain(..).collect()
            }
            Err(_) => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MoodReactionService;

    #[test]
    fn drain_retains_reactions_until_ui_ready() {
        let service = MoodReactionService::new();
        service.set("opencode-working", true);

        assert!(service.drain().is_empty());

        service.mark_ui_ready();
        let drained = service.drain();
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0].trigger, "opencode-working");
        assert!(drained[0].sticky);
        assert!(service.drain().is_empty());
    }
}
