use std::sync::{Arc, Mutex};
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

impl Scheduler {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(Vec::new())),
            wake: Arc::new(Notify::new()),
            shutdown: CancellationToken::new(),
        }
    }

    pub fn set(
        &self,
        owner: &str,
        key: &str,
        interval_secs: u64,
        repeat: bool,
    ) -> Result<(), SchedulerError> {
        if interval_secs == 0 {
            return Err(SchedulerError::ZeroInterval);
        }

        let interval = Duration::from_secs(interval_secs);
        let mut entries = self.entries.lock().expect("scheduler entries lock poisoned");
        entries.retain(|entry| !(entry.owner == owner && entry.key == key));
        entries.push(ScheduleEntry {
            owner: owner.to_string(),
            key: key.to_string(),
            interval,
            repeat,
            next_fire_at: Instant::now() + interval,
        });
        entries.sort_by_key(|entry| entry.next_fire_at);
        drop(entries);
        self.wake.notify_one();
        Ok(())
    }

    pub fn cancel(&self, owner: &str, key: &str) {
        let mut entries = self.entries.lock().expect("scheduler entries lock poisoned");
        entries.retain(|entry| !(entry.owner == owner && entry.key == key));
        drop(entries);
        self.wake.notify_one();
    }

    pub fn cancel_all(&self, owner: &str) {
        let mut entries = self.entries.lock().expect("scheduler entries lock poisoned");
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
        let entries = Arc::clone(&self.entries);
        let wake = Arc::clone(&self.wake);
        let shutdown = self.shutdown.clone();
        let on_fire = Arc::new(on_fire);

        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()
                .expect("scheduler runtime should build");

            runtime.block_on(async move {
                loop {
                    let next_wait = {
                        let entries = entries.lock().expect("scheduler entries lock poisoned");
                        entries.first().map(|entry| {
                            entry
                                .next_fire_at
                                .checked_duration_since(Instant::now())
                                .unwrap_or(Duration::ZERO)
                        })
                    };

                    match next_wait {
                        Some(wait) => {
                            tokio::select! {
                                _ = shutdown.cancelled() => break,
                                _ = wake.notified() => continue,
                                _ = tokio::time::sleep(wait) => {}
                            }
                        }
                        None => {
                            tokio::select! {
                                _ = shutdown.cancelled() => break,
                                _ = wake.notified() => continue,
                            }
                        }
                    }

                    let due_entries = {
                        let now = Instant::now();
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
