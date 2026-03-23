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
    pub target_epoch_secs: Option<u64>,
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
    /// Whether the UI has signalled that it is mounted and listening for badge
    /// events.  Until this is `true`, `take_if_changed` withholds data so the
    /// background flush loop does not consume and discard badges before the
    /// frontend can receive them.
    ui_ready: bool,
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
                ui_ready: false,
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

    /// Remove all badge items for `source_plugin`. Marks state as dirty.
    pub fn clear(&self, source_plugin: &str) {
        if let Ok(mut state) = self.inner.lock() {
            let removed = state.badges.remove(source_plugin).is_some();
            if removed {
                state.dirty = true;
            }
        }
    }

    /// Force the current merged badge list to be emitted on the next flush.
    pub fn refresh(&self) {
        if let Ok(mut state) = self.inner.lock() {
            state.dirty = true;
        }
    }

    /// Signal that the UI has mounted and is listening for badge events.
    ///
    /// Any badges already buffered will be emitted on the next flush tick.
    pub fn mark_ui_ready(&self) {
        if let Ok(mut state) = self.inner.lock() {
            state.ui_ready = true;
            if !state.badges.is_empty() {
                state.dirty = true;
            }
        }
    }

    /// Return the merged badge list if anything changed since the last call.
    ///
    /// Returns `None` when the UI has not yet signalled readiness (via
    /// [`mark_ui_ready`]) so that early badge pushes are not consumed and
    /// discarded before the frontend can receive them.
    pub fn take_if_changed(&self) -> Option<Vec<PeekBadgeItem>> {
        let mut state = self.inner.lock().ok()?;
        if !state.ui_ready || !state.dirty {
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

#[cfg(test)]
mod tests {
    use super::{PeekBadgeItem, PeekBadgeService};

    #[test]
    fn clear_removes_plugin_badges_from_merged_output() {
        let service = PeekBadgeService::new();
        service.mark_ui_ready();
        service.set(
            "plugin-a",
            vec![PeekBadgeItem {
                label: "A".into(),
                value: "one".into(),
                icon: None,
                target_epoch_secs: None,
            }],
        );
        service.set(
            "plugin-b",
            vec![PeekBadgeItem {
                label: "B".into(),
                value: "two".into(),
                icon: None,
                target_epoch_secs: None,
            }],
        );

        let merged = service.take_if_changed().expect("initial badges");
        assert_eq!(merged.len(), 2);

        service.clear("plugin-a");
        let merged = service.take_if_changed().expect("updated badges");
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].label, "B");
    }
}
