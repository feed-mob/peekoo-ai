use serde::{Deserialize, Serialize};

/// Information about an active schedule timer.
///
/// Returned by [`crate::peekoo::schedule::get`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScheduleInfo {
    /// The plugin that owns this schedule.
    pub owner: String,
    /// The schedule key (unique per plugin).
    pub key: String,
    /// Interval in seconds between firings.
    pub interval_secs: u64,
    /// Whether the schedule repeats after firing.
    pub repeat: bool,
    /// Seconds remaining until the next firing.
    pub time_remaining_secs: u64,
}

/// A single badge item displayed on the Peek overlay.
///
/// Pass a slice of these to [`crate::peekoo::badge::set`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BadgeItem {
    /// Short label shown on the badge.
    pub label: String,
    /// Display value (e.g. "~5 min", "3").
    pub value: String,
    /// Optional Lucide icon name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Optional countdown in seconds (UI will tick it down live).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub countdown_secs: Option<u64>,
}

/// A filesystem entry returned by [`crate::peekoo::fs::read_dir`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FsEntry {
    pub name: String,
    pub is_dir: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_secs: Option<u64>,
}

/// Known system events that Peekoo delivers to plugins.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SystemEvent {
    /// A scheduled timer fired. Payload contains `{ "key": "<schedule_key>" }`.
    ScheduleFired,
    /// The system resumed from sleep/suspend.
    SystemWake,
}

impl SystemEvent {
    /// Returns the event name string used in the plugin manifest and `on_event` calls.
    pub fn as_str(&self) -> &'static str {
        match self {
            SystemEvent::ScheduleFired => "schedule:fired",
            SystemEvent::SystemWake => "system:wake",
        }
    }
}
