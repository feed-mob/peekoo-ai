use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use std::time::{Duration, Instant};

use serde::Serialize;
use thiserror::Error;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScheduleInfo {
    pub owner: String,
    pub key: String,
    pub interval_secs: u64,
    pub repeat: bool,
    pub time_remaining_secs: u64,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SchedulerError {
    #[error("interval must be greater than zero")]
    ZeroInterval,
}

#[derive(Debug, Clone)]
struct ScheduleEntry {
    owner: String,
    key: String,
    interval: Duration,
    repeat: bool,
    next_fire_at: Instant,
}

#[derive(Clone)]
pub struct Scheduler {
    entries: Arc<Mutex<Vec<ScheduleEntry>>>,
    wake: Arc<Notify>,
    shutdown: CancellationToken,
}

const WAKE_DRIFT_TOLERANCE: Duration = Duration::from_secs(30);

impl Scheduler {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(Vec::new())),
            wake: Arc::new(Notify::new()),
            shutdown: CancellationToken::new(),
        }
    }

    /// Register a schedule entry.
    ///
    /// `delay_secs` overrides how long until the *first* fire. Subsequent
    /// repeat fires always use `interval_secs`. Pass `None` to use the full
    /// interval for the first fire (the original behaviour).  Pass `Some(0)`
    /// to fire as soon as possible.
    pub fn set(
        &self,
        owner: &str,
        key: &str,
        interval_secs: u64,
        repeat: bool,
        delay_secs: Option<u64>,
    ) -> Result<(), SchedulerError> {
        if interval_secs == 0 {
            return Err(SchedulerError::ZeroInterval);
        }

        let interval = Duration::from_secs(interval_secs);
        let first_delay = Duration::from_secs(delay_secs.unwrap_or(interval_secs));
        let mut entries = self
            .entries
            .lock()
            .expect("scheduler entries lock poisoned");
        entries.retain(|entry| !(entry.owner == owner && entry.key == key));
        entries.push(ScheduleEntry {
            owner: owner.to_string(),
            key: key.to_string(),
            interval,
            repeat,
            next_fire_at: Instant::now() + first_delay,
        });
        entries.sort_by_key(|entry| entry.next_fire_at);
        drop(entries);
        self.wake.notify_one();
        Ok(())
    }

    pub fn cancel(&self, owner: &str, key: &str) {
        let mut entries = self
            .entries
            .lock()
            .expect("scheduler entries lock poisoned");
        entries.retain(|entry| !(entry.owner == owner && entry.key == key));
        drop(entries);
        self.wake.notify_one();
    }

    pub fn cancel_all(&self, owner: &str) {
        let mut entries = self
            .entries
            .lock()
            .expect("scheduler entries lock poisoned");
        entries.retain(|entry| entry.owner != owner);
        drop(entries);
        self.wake.notify_one();
    }

    pub fn list(&self, owner: &str) -> Vec<ScheduleInfo> {
        let now = Instant::now();
        self.entries
            .lock()
            .expect("scheduler entries lock poisoned")
            .iter()
            .filter(|entry| entry.owner == owner)
            .map(|entry| ScheduleInfo {
                owner: entry.owner.clone(),
                key: entry.key.clone(),
                interval_secs: entry.interval.as_secs(),
                repeat: entry.repeat,
                time_remaining_secs: entry
                    .next_fire_at
                    .checked_duration_since(now)
                    .unwrap_or(Duration::ZERO)
                    .as_secs(),
            })
            .collect()
    }

    pub fn shutdown_token(&self) -> CancellationToken {
        self.shutdown.clone()
    }

    pub fn start<F>(&self, on_fire: F) -> std::thread::JoinHandle<()>
    where
        F: Fn(String, String) + Send + Sync + 'static,
    {
        self.start_with_wake_handler(on_fire, |_| {})
    }

    pub fn start_with_wake_handler<F, G>(
        &self,
        on_fire: F,
        on_wake: G,
    ) -> std::thread::JoinHandle<()>
    where
        F: Fn(String, String) + Send + Sync + 'static,
        G: Fn(String) + Send + Sync + 'static,
    {
        let entries = Arc::clone(&self.entries);
        let wake = Arc::clone(&self.wake);
        let shutdown = self.shutdown.clone();
        let on_fire = Arc::new(on_fire);
        let on_wake = Arc::new(on_wake);

        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()
                .expect("scheduler runtime should build");

            runtime.block_on(async move {
                let mut last_monotonic = Instant::now();
                let mut last_wall = SystemTime::now();

                loop {
                    // Register the wake listener *before* releasing the entries
                    // lock so that a concurrent `set()` / `cancel()` that calls
                    // `notify_one()` between the lock-drop and the `select!`
                    // poll is never lost.
                    let (next_wait, notified) = {
                        let entries = entries.lock().expect("scheduler entries lock poisoned");
                        let notified = wake.notified();
                        let wait = entries.first().map(|entry| {
                            entry
                                .next_fire_at
                                .checked_duration_since(Instant::now())
                                .unwrap_or(Duration::ZERO)
                        });
                        (wait, notified)
                    };

                    match next_wait {
                        Some(wait) => {
                            tokio::select! {
                                _ = shutdown.cancelled() => break,
                                _ = notified => continue,
                                _ = tokio::time::sleep(wait) => {}
                            }
                        }
                        None => {
                            tokio::select! {
                                _ = shutdown.cancelled() => break,
                                _ = notified => continue,
                            }
                        }
                    }

                    let now_monotonic = Instant::now();
                    let now_wall = SystemTime::now();
                    let wake_detected = detect_wake_drift(
                        last_monotonic,
                        last_wall,
                        now_monotonic,
                        now_wall,
                        WAKE_DRIFT_TOLERANCE,
                    );
                    last_monotonic = now_monotonic;
                    last_wall = now_wall;

                    if wake_detected {
                        let owners = scheduled_owners(&entries);
                        for owner in owners {
                            on_wake(owner);
                        }
                        continue;
                    }

                    let due_entries = {
                        let now = now_monotonic;
                        let mut entries = entries.lock().expect("scheduler entries lock poisoned");
                        let mut due = Vec::new();
                        let mut pending = Vec::with_capacity(entries.len());
                        for mut entry in entries.drain(..) {
                            if entry.next_fire_at <= now {
                                due.push((entry.owner.clone(), entry.key.clone()));
                                if entry.repeat {
                                    entry.next_fire_at = now + entry.interval;
                                    pending.push(entry);
                                }
                            } else {
                                pending.push(entry);
                            }
                        }
                        pending.sort_by_key(|entry| entry.next_fire_at);
                        *entries = pending;
                        due
                    };

                    for (owner, key) in due_entries {
                        on_fire(owner, key);
                    }
                }
            })
        })
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

fn scheduled_owners(entries: &Arc<Mutex<Vec<ScheduleEntry>>>) -> Vec<String> {
    let entries = entries.lock().expect("scheduler entries lock poisoned");
    entries
        .iter()
        .map(|entry| entry.owner.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn detect_wake_drift(
    previous_monotonic: Instant,
    previous_wall: SystemTime,
    current_monotonic: Instant,
    current_wall: SystemTime,
    tolerance: Duration,
) -> bool {
    let monotonic_elapsed = current_monotonic
        .checked_duration_since(previous_monotonic)
        .unwrap_or(Duration::ZERO);
    let Ok(wall_elapsed) = current_wall.duration_since(previous_wall) else {
        return false;
    };

    wall_elapsed > monotonic_elapsed.saturating_add(tolerance)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant, SystemTime};

    use super::detect_wake_drift;

    #[test]
    fn detects_large_wall_clock_jump_as_wake() {
        let monotonic_start = Instant::now();
        let wall_start = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);

        assert!(detect_wake_drift(
            monotonic_start,
            wall_start,
            monotonic_start + Duration::from_secs(5),
            wall_start + Duration::from_secs(50),
            Duration::from_secs(30),
        ));
    }

    #[test]
    fn does_not_detect_normal_clock_progress_as_wake() {
        let monotonic_start = Instant::now();
        let wall_start = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);

        assert!(!detect_wake_drift(
            monotonic_start,
            wall_start,
            monotonic_start + Duration::from_secs(5),
            wall_start + Duration::from_secs(10),
            Duration::from_secs(30),
        ));
    }

    #[test]
    fn ignores_backward_wall_clock_adjustments() {
        let monotonic_start = Instant::now();
        let wall_start = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);

        assert!(!detect_wake_drift(
            monotonic_start,
            wall_start,
            monotonic_start + Duration::from_secs(5),
            wall_start - Duration::from_secs(60),
            Duration::from_secs(30),
        ));
    }
}
