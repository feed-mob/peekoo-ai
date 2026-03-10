use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

/// A single status badge item that a plugin wants displayed on the sprite.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PeekBadgeItem {
    pub label: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub countdown_secs: Option<u64>,
}

/// Collects badge updates from plugins and merges them into a single list.
///
/// Each plugin overwrites its own badge set via `set`. Consumers call `take_if_changed`
/// to get the merged list only when something has actually changed since the last read.
pub struct PeekBadgeService {
    inner: Mutex<BadgeState>,
}

struct BadgeState {
    badges: HashMap<String, Vec<PeekBadgeItem>>,
    dirty: bool,
}

impl Default for PeekBadgeService {
    fn default() -> Self {
        Self::new()
    }
}

impl PeekBadgeService {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(BadgeState {
                badges: HashMap::new(),
                dirty: false,
            }),
        }
    }

    /// Replace all badge items for `source_plugin`. Marks state as dirty.
    pub fn set(&self, source_plugin: &str, items: Vec<PeekBadgeItem>) {
        if let Ok(mut state) = self.inner.lock() {
            state.badges.insert(source_plugin.to_string(), items);
            state.dirty = true;
        }
    }

    /// Return the merged badge list if anything changed since the last call.
    /// Clears the dirty flag so subsequent calls return `None` until the next `set`.
    pub fn take_if_changed(&self) -> Option<Vec<PeekBadgeItem>> {
        let mut state = self.inner.lock().ok()?;
        if !state.dirty {
            return None;
        }
        state.dirty = false;

        let mut merged: Vec<PeekBadgeItem> = state
            .badges
            .values()
            .flat_map(|items| items.iter().cloned())
            .collect();

        // Deterministic order: sort by label so the frontend rotation is stable.
        merged.sort_by(|a, b| a.label.cmp(&b.label));
        Some(merged)
    }
}
