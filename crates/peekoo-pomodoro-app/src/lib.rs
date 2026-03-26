use std::sync::{Arc, Mutex, MutexGuard};

use chrono::{DateTime, Utc};
use peekoo_notifications::{
    MoodReactionService, Notification, NotificationService, PeekBadgeItem, PeekBadgeService,
};
use peekoo_pomodoro_domain::{PomodoroMode, PomodoroSettings, PomodoroState, PomodoroStatus};
use peekoo_scheduler::Scheduler;
use rusqlite::{Connection, params};
use serde::Serialize;
use uuid::Uuid;

const POMODORO_OWNER: &str = "pomodoro";
const POMODORO_TIMER_KEY: &str = "active-timer";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PomodoroStatusDto {
    pub mode: String,
    pub state: String,
    pub minutes: u32,
    pub time_remaining_secs: u64,
    pub completed_focus: u32,
    pub completed_breaks: u32,
    pub enable_memo: bool,
    pub auto_advance: bool,
    pub default_work_minutes: u32,
    pub default_break_minutes: u32,
    pub long_break_minutes: u32,
    pub long_break_interval: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PomodoroCycleDto {
    pub id: String,
    pub mode: String,
    pub planned_minutes: u32,
    pub actual_elapsed_secs: u64,
    pub outcome: String,
    pub started_at: String,
    pub ended_at: String,
    pub memo_requested: bool,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PomodoroSettingsInput {
    pub work_minutes: u32,
    pub break_minutes: u32,
    pub long_break_minutes: u32,
    pub long_break_interval: u32,
    pub enable_memo: bool,
    pub auto_advance: bool,
}

pub struct PomodoroAppService {
    conn: Arc<Mutex<Connection>>,
    scheduler: Scheduler,
    notifications: Arc<NotificationService>,
    peek_badges: Arc<PeekBadgeService>,
    mood_reactions: Arc<MoodReactionService>,
}

impl PomodoroAppService {
    pub fn new(
        conn: Arc<Mutex<Connection>>,
        notifications: Arc<NotificationService>,
        peek_badges: Arc<PeekBadgeService>,
        mood_reactions: Arc<MoodReactionService>,
    ) -> Result<Self, String> {
        let scheduler = Scheduler::new();
        let service = Self {
            conn,
            scheduler: scheduler.clone(),
            notifications,
            peek_badges,
            mood_reactions,
        };

        service.ensure_seed_row()?;
        service.start_scheduler_loop();
        service.reconcile_runtime_state()?;
        Ok(service)
    }

    pub fn get_status(&self) -> Result<PomodoroStatusDto, String> {
        // Check if we need to reset daily counters before returning status
        let conn = self.conn()?;
        let mut status = load_status(&conn)?;

        let today = chrono::Local::now().date_naive().to_string();
        tracing::info!(
            "Pomodoro get_status: today={}, last_reset_date={:?}, completed_focus={}, completed_breaks={}",
            today,
            status.last_reset_date,
            status.completed_focus,
            status.completed_breaks
        );

        if status.last_reset_date.as_ref() != Some(&today) {
            tracing::info!(
                "Resetting daily counters from {} to {}",
                status.last_reset_date.as_deref().unwrap_or("None"),
                today
            );
            status.completed_focus = 0;
            status.completed_breaks = 0;
            status.last_reset_date = Some(today);
            save_status(&conn, &status)?;
        }
        drop(conn);

        let status = self.refresh_runtime_if_due()?;
        self.publish_badges(&status);
        Ok(status_to_dto(&status))
    }

    pub fn set_settings(&self, input: PomodoroSettingsInput) -> Result<PomodoroStatusDto, String> {
        let settings = PomodoroSettings::new(
            input.work_minutes,
            input.break_minutes,
            input.long_break_minutes,
            input.long_break_interval,
            input.enable_memo,
            input.auto_advance,
        )
        .map_err(|err| err.to_string())?;

        let conn = self.conn()?;
        let mut status = load_status(&conn)?;
        status.set_settings(settings);
        save_status(&conn, &status)?;
        drop(conn);

        self.sync_scheduler(&status)?;
        self.publish_badges(&status);
        Ok(status_to_dto(&status))
    }

    pub fn start(&self, mode: &str, minutes: u32) -> Result<PomodoroStatusDto, String> {
        let mode = parse_mode(mode)?;
        let conn = self.conn()?;
        let mut status = load_status(&conn)?;

        if matches!(status.state, PomodoroState::Running | PomodoroState::Paused) {
            return Err("pomodoro is already active".to_string());
        }

        let now = now_epoch();
        status
            .start(mode, minutes, now)
            .map_err(|err| err.to_string())?;
        save_status(&conn, &status)?;
        drop(conn);

        self.sync_scheduler(&status)?;
        self.publish_badges(&status);
        self.publish_start_mood(&status);
        Ok(status_to_dto(&status))
    }

    pub fn pause(&self) -> Result<PomodoroStatusDto, String> {
        let conn = self.conn()?;
        let mut status = load_status(&conn)?;
        let now = now_epoch();
        status.pause(now).map_err(|err| err.to_string())?;
        save_status(&conn, &status)?;
        drop(conn);

        self.sync_scheduler(&status)?;
        self.publish_badges(&status);
        self.publish_break_mood();
        Ok(status_to_dto(&status))
    }

    pub fn resume(&self) -> Result<PomodoroStatusDto, String> {
        let conn = self.conn()?;
        let mut status = load_status(&conn)?;
        let now = now_epoch();
        status.resume(now).map_err(|err| err.to_string())?;
        save_status(&conn, &status)?;
        drop(conn);

        self.sync_scheduler(&status)?;
        self.publish_badges(&status);
        self.publish_start_mood(&status);
        Ok(status_to_dto(&status))
    }

    pub fn finish(&self) -> Result<PomodoroStatusDto, String> {
        let conn = self.conn()?;
        let mut status = load_status(&conn)?;

        if matches!(status.state, PomodoroState::Running | PomodoroState::Paused) {
            let record = status.finish(now_epoch()).map_err(|err| err.to_string())?;
            insert_cycle_record(&conn, &record)?;
            save_status(&conn, &status)?;
        } else {
            status
                .switch_mode(status.mode)
                .map_err(|err| err.to_string())?;
            save_status(&conn, &status)?;
        }
        drop(conn);

        self.sync_scheduler(&status)?;
        self.publish_badges(&status);
        self.publish_break_mood();
        Ok(status_to_dto(&status))
    }

    pub fn switch_mode(&self, mode: &str) -> Result<PomodoroStatusDto, String> {
        let mode = parse_mode(mode)?;
        let conn = self.conn()?;
        let mut status = load_status(&conn)?;

        if matches!(status.state, PomodoroState::Running | PomodoroState::Paused) {
            let record = status.finish(now_epoch()).map_err(|err| err.to_string())?;
            insert_cycle_record(&conn, &record)?;
        }

        status.switch_mode(mode).map_err(|err| err.to_string())?;
        save_status(&conn, &status)?;
        drop(conn);

        self.sync_scheduler(&status)?;
        self.publish_badges(&status);
        Ok(status_to_dto(&status))
    }

    pub fn save_pomodoro_memo(
        &self,
        id: Option<String>,
        memo: String,
    ) -> Result<PomodoroStatusDto, String> {
        let conn = self.conn()?;

        if let Some(cycle_id) = id {
            conn.execute(
                "UPDATE pomodoro_cycle_history SET memo = ?1 WHERE id = ?2",
                params![memo, cycle_id],
            )
            .map_err(|err| format!("Failed to update specific cycle memo: {err}"))?;
        } else {
            conn.execute(
                "UPDATE pomodoro_cycle_history SET memo = ?1 WHERE mode = 'work' AND id = (SELECT id FROM pomodoro_cycle_history WHERE mode = 'work' ORDER BY ended_at DESC LIMIT 1)",
                params![memo],
            )
            .map_err(|err| format!("Failed to update latest cycle memo: {err}"))?;
        }

        let status = load_status(&conn)?;
        Ok(status_to_dto(&status))
    }

    pub fn history(&self, limit: usize) -> Result<Vec<PomodoroCycleDto>, String> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, mode, planned_minutes, actual_elapsed_secs, outcome, started_at, ended_at, memo_requested, memo FROM pomodoro_cycle_history ORDER BY ended_at DESC LIMIT ?1",
            )
            .map_err(|err| format!("Prepare pomodoro history query error: {err}"))?;

        let rows = stmt
            .query_map(params![limit as i64], |row| {
                Ok(PomodoroCycleDto {
                    id: row.get(0)?,
                    mode: row.get(1)?,
                    planned_minutes: row.get::<_, i64>(2)? as u32,
                    actual_elapsed_secs: row.get::<_, i64>(3)? as u64,
                    outcome: row.get(4)?,
                    started_at: row.get(5)?,
                    ended_at: row.get(6)?,
                    memo_requested: row.get::<_, i64>(7)? == 1,
                    memo: row.get(8)?,
                })
            })
            .map_err(|err| format!("Query pomodoro history error: {err}"))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|err| format!("Collect pomodoro history error: {err}"))
    }

    pub fn history_by_date_range(
        &self,
        start_date: &str,
        end_date: &str,
        limit: usize,
    ) -> Result<Vec<PomodoroCycleDto>, String> {
        tracing::info!(
            "Pomodoro history_by_date_range: start_date={}, end_date={}, limit={}",
            start_date,
            end_date,
            limit
        );

        let conn = self.conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, mode, planned_minutes, actual_elapsed_secs, outcome, started_at, ended_at, memo_requested, memo 
                 FROM pomodoro_cycle_history 
                 WHERE date(datetime(ended_at, 'localtime')) BETWEEN ?1 AND ?2 
                 ORDER BY ended_at DESC 
                 LIMIT ?3",
            )
            .map_err(|err| format!("Prepare pomodoro history by date query error: {err}"))?;

        let rows = stmt
            .query_map(params![start_date, end_date, limit as i64], |row| {
                Ok(PomodoroCycleDto {
                    id: row.get(0)?,
                    mode: row.get(1)?,
                    planned_minutes: row.get::<_, i64>(2)? as u32,
                    actual_elapsed_secs: row.get::<_, i64>(3)? as u64,
                    outcome: row.get(4)?,
                    started_at: row.get(5)?,
                    ended_at: row.get(6)?,
                    memo_requested: row.get::<_, i64>(7)? == 1,
                    memo: row.get(8)?,
                })
            })
            .map_err(|err| format!("Query pomodoro history by date error: {err}"))?;

        let result = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| format!("Collect pomodoro history by date error: {err}"))?;

        tracing::info!(
            "Pomodoro history_by_date_range: returned {} records. Breakdown by mode/outcome:",
            result.len()
        );
        let work_completed = result
            .iter()
            .filter(|r| r.mode == "work" && r.outcome == "completed")
            .count();
        let work_cancelled = result
            .iter()
            .filter(|r| r.mode == "work" && r.outcome == "cancelled")
            .count();
        let break_completed = result
            .iter()
            .filter(|r| r.mode == "break" && r.outcome == "completed")
            .count();
        let break_cancelled = result
            .iter()
            .filter(|r| r.mode == "break" && r.outcome == "cancelled")
            .count();
        tracing::info!(
            "  work: {} completed, {} cancelled | break: {} completed, {} cancelled",
            work_completed,
            work_cancelled,
            break_completed,
            break_cancelled
        );

        Ok(result)
    }

    fn conn(&self) -> Result<MutexGuard<'_, Connection>, String> {
        self.conn
            .lock()
            .map_err(|err| format!("Pomodoro db lock error: {err}"))
    }

    fn ensure_seed_row(&self) -> Result<(), String> {
        let conn = self.conn()?;
        conn.execute(
            concat!(
                "INSERT OR IGNORE INTO pomodoro_state (",
                "id, mode, state, minutes, time_remaining_secs, started_at_epoch, expected_fire_at_epoch, ",
                "default_work_minutes, default_break_minutes, long_break_minutes, long_break_interval, ",
                "enable_memo, auto_advance, completed_focus, completed_breaks, last_reset_date, updated_at",
                ") VALUES (1, 'work', 'Idle', 25, 1500, NULL, NULL, 25, 5, 15, 4, 0, 0, 0, 0, date('now'), datetime('now'))"
            ),
            [],
        )
        .map_err(|err| format!("Seed pomodoro state error: {err}"))?;
        Ok(())
    }

    fn start_scheduler_loop(&self) {
        let conn = Arc::clone(&self.conn);
        let notifications = Arc::clone(&self.notifications);
        let peek_badges = Arc::clone(&self.peek_badges);
        let mood_reactions = Arc::clone(&self.mood_reactions);
        let wake_conn = Arc::clone(&self.conn);
        let wake_peek_badges = Arc::clone(&self.peek_badges);

        self.scheduler.start_with_wake_handler(
            move |owner, key| {
                if owner != POMODORO_OWNER || key != POMODORO_TIMER_KEY {
                    return;
                }

                if let Err(err) =
                    complete_due_session(&conn, &notifications, &peek_badges, &mood_reactions)
                {
                    tracing::warn!("Pomodoro scheduled completion failed: {err}");
                }
            },
            move |owner| {
                if owner != POMODORO_OWNER {
                    return;
                }

                if let Err(err) = reconcile_after_wake(&wake_conn, &wake_peek_badges) {
                    tracing::warn!("Pomodoro wake reconciliation failed: {err}");
                }
            },
        );
    }

    fn reconcile_runtime_state(&self) -> Result<(), String> {
        let conn = self.conn()?;
        let mut status = load_status(&conn)?;

        // Check if we need to reset daily counters (use local timezone)
        let today = chrono::Local::now().date_naive().to_string();
        if status.last_reset_date.as_ref() != Some(&today) {
            status.completed_focus = 0;
            status.completed_breaks = 0;
            status.last_reset_date = Some(today);
            save_status(&conn, &status)?;
        }
        drop(conn);

        let status = self.refresh_runtime_if_due()?;
        self.sync_scheduler(&status)?;
        self.publish_badges(&status);
        Ok(())
    }

    fn refresh_runtime_if_due(&self) -> Result<PomodoroStatus, String> {
        let conn = self.conn()?;
        let mut status = load_status(&conn)?;

        if status.state == PomodoroState::Running {
            let now = now_epoch();
            status.refresh_time_remaining(now);

            if status.time_remaining_secs == 0 {
                let record = status.complete(now).map_err(|err| err.to_string())?;
                insert_cycle_record(&conn, &record)?;

                if status.settings.auto_advance {
                    let next_mode = match status.mode {
                        PomodoroMode::Work => {
                            if status.completed_focus > 0
                                && status.completed_focus % status.settings.long_break_interval == 0
                            {
                                PomodoroMode::Break // Still Break, but we'll use long duration
                            } else {
                                PomodoroMode::Break
                            }
                        }
                        PomodoroMode::Break => PomodoroMode::Work,
                    };

                    let next_minutes = match next_mode {
                        PomodoroMode::Work => status.settings.default_work_minutes,
                        PomodoroMode::Break => {
                            if status.mode == PomodoroMode::Work
                                && status.completed_focus > 0
                                && status.completed_focus % status.settings.long_break_interval == 0
                            {
                                status.settings.long_break_minutes
                            } else {
                                status.settings.default_break_minutes
                            }
                        }
                    };

                    status
                        .start(next_mode, next_minutes, now)
                        .map_err(|err| err.to_string())?;
                }

                save_status(&conn, &status)?;
                drop(conn);

                self.sync_scheduler(&status)?;
                self.publish_badges(&status);

                if status.state == PomodoroState::Running {
                    self.publish_start_mood(&status);
                } else {
                    self.publish_completion_side_effects(&status, &record.mode);
                }
                return Ok(status);
            }
        }

        Ok(status)
    }

    fn sync_scheduler(&self, status: &PomodoroStatus) -> Result<(), String> {
        self.scheduler.cancel(POMODORO_OWNER, POMODORO_TIMER_KEY);

        if status.state == PomodoroState::Running && status.time_remaining_secs > 0 {
            self.scheduler
                .set(
                    POMODORO_OWNER,
                    POMODORO_TIMER_KEY,
                    status.time_remaining_secs,
                    false,
                    Some(status.time_remaining_secs),
                )
                .map_err(|err| err.to_string())?;
        }

        Ok(())
    }

    fn publish_badges(&self, status: &PomodoroStatus) {
        if matches!(status.state, PomodoroState::Running | PomodoroState::Paused) {
            let icon = Some(match status.mode {
                PomodoroMode::Work => "brain".to_string(),
                PomodoroMode::Break => "coffee".to_string(),
            });
            let label = match (status.mode, status.state) {
                (PomodoroMode::Work, PomodoroState::Paused) => "Focus (Paused)".to_string(),
                (PomodoroMode::Break, PomodoroState::Paused) => "Break (Paused)".to_string(),
                (PomodoroMode::Work, _) => "Focus".to_string(),
                (PomodoroMode::Break, _) => "Break".to_string(),
            };
            let countdown_secs =
                (status.state == PomodoroState::Running).then_some(status.time_remaining_secs);

            self.peek_badges.set(
                POMODORO_OWNER,
                vec![PeekBadgeItem {
                    label,
                    value: format_countdown(status.time_remaining_secs),
                    icon,
                    countdown_secs,
                }],
            );
        } else {
            self.peek_badges.clear(POMODORO_OWNER);
        }
    }

    fn publish_start_mood(&self, status: &PomodoroStatus) {
        let trigger = match status.mode {
            PomodoroMode::Work => "pomodoro-started",
            PomodoroMode::Break => "pomodoro-resting",
        };
        self.mood_reactions.set(trigger, true);
    }

    fn publish_break_mood(&self) {
        self.mood_reactions.set("pomodoro-break", false);
    }

    fn publish_completion_side_effects(&self, status: &PomodoroStatus, mode: &PomodoroMode) {
        self.mood_reactions.set("pomodoro-completed", false);

        let (title, body) = match mode {
            PomodoroMode::Work => (
                "Focus Session Complete",
                "Great job! Time to take a short break.",
            ),
            PomodoroMode::Break => ("Break Complete", "Ready to start focusing again?"),
        };

        let _ = self.notifications.notify(Notification {
            source: POMODORO_OWNER.to_string(),
            title: title.to_string(),
            body: body.to_string(),
            action_url: None,
            action_label: None,
            panel_label: None,
        });

        self.publish_badges(status);
    }
}

fn load_status(conn: &Connection) -> Result<PomodoroStatus, String> {
    conn.query_row(
        concat!(
            "SELECT mode, state, minutes, time_remaining_secs, started_at_epoch, expected_fire_at_epoch, ",
            "default_work_minutes, default_break_minutes, enable_memo, completed_focus, completed_breaks, ",
            "long_break_minutes, long_break_interval, auto_advance, last_reset_date ",
            "FROM pomodoro_state WHERE id = 1"
        ),
        [],
        |row| {
            let settings = PomodoroSettings::new(
                row.get::<_, i64>(6)? as u32,
                row.get::<_, i64>(7)? as u32,
                row.get::<_, i64>(11)? as u32,
                row.get::<_, i64>(12)? as u32,
                row.get::<_, i64>(8)? == 1,
                row.get::<_, i64>(13)? == 1,
            )
            .map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(
                    6,
                    rusqlite::types::Type::Integer,
                    Box::new(std::io::Error::other(err.to_string())),
                )
            })?;

            Ok(PomodoroStatus {
                mode: parse_mode(&row.get::<_, String>(0)?).map_err(to_from_sql_error)?,
                state: parse_state(&row.get::<_, String>(1)?).map_err(to_from_sql_error)?,
                minutes: row.get::<_, i64>(2)? as u32,
                time_remaining_secs: row.get::<_, i64>(3)? as u64,
                started_at_epoch: row.get(4)?,
                expected_fire_at_epoch: row.get(5)?,
                settings,
                completed_focus: row.get::<_, i64>(9)? as u32,
                completed_breaks: row.get::<_, i64>(10)? as u32,
                last_reset_date: row.get(14)?,
            })
        },
    )
    .map_err(|err| format!("Load pomodoro state error: {err}"))
}

fn save_status(conn: &Connection, status: &PomodoroStatus) -> Result<(), String> {
    conn.execute(
        concat!(
            "UPDATE pomodoro_state SET ",
            "mode = ?1, state = ?2, minutes = ?3, time_remaining_secs = ?4, started_at_epoch = ?5, expected_fire_at_epoch = ?6, ",
            "default_work_minutes = ?7, default_break_minutes = ?8, enable_memo = ?9, completed_focus = ?10, completed_breaks = ?11, ",
            "long_break_minutes = ?12, long_break_interval = ?13, auto_advance = ?14, last_reset_date = ?15, updated_at = datetime('now') ",
            "WHERE id = 1"
        ),
        params![
            status.mode.as_str(),
            status.state.as_str(),
            status.minutes as i64,
            status.time_remaining_secs as i64,
            status.started_at_epoch,
            status.expected_fire_at_epoch,
            status.settings.default_work_minutes as i64,
            status.settings.default_break_minutes as i64,
            i64::from(status.settings.enable_memo),
            status.completed_focus as i64,
            status.completed_breaks as i64,
            status.settings.long_break_minutes as i64,
            status.settings.long_break_interval as i64,
            i64::from(status.settings.auto_advance),
            status.last_reset_date.as_deref(),
        ],
    )
    .map_err(|err| format!("Save pomodoro state error: {err}"))?;
    Ok(())
}

fn insert_cycle_record(
    conn: &Connection,
    record: &peekoo_pomodoro_domain::PomodoroCycleRecord,
) -> Result<(), String> {
    conn.execute(
        concat!(
            "INSERT INTO pomodoro_cycle_history ",
            "(id, mode, planned_minutes, actual_elapsed_secs, outcome, started_at, ended_at, memo_requested) ",
            "VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
        ),
        params![
            Uuid::new_v4().to_string(),
            record.mode.as_str(),
            record.planned_minutes as i64,
            record.actual_elapsed_secs as i64,
            record.outcome.as_str(),
            iso_from_epoch(record.started_at_epoch)?,
            iso_from_epoch(record.ended_at_epoch)?,
            i64::from(record.memo_requested),
        ],
    )
    .map_err(|err| format!("Insert pomodoro cycle record error: {err}"))?;
    Ok(())
}

fn complete_due_session(
    conn: &Arc<Mutex<Connection>>,
    notifications: &Arc<NotificationService>,
    peek_badges: &Arc<PeekBadgeService>,
    mood_reactions: &Arc<MoodReactionService>,
) -> Result<(), String> {
    let conn = conn
        .lock()
        .map_err(|err| format!("Pomodoro db lock error: {err}"))?;
    let mut status = load_status(&conn)?;
    if status.state != PomodoroState::Running {
        return Ok(());
    }

    let record = status
        .complete(now_epoch())
        .map_err(|err| err.to_string())?;
    insert_cycle_record(&conn, &record)?;

    if status.settings.auto_advance {
        let now = now_epoch();
        let next_mode = match status.mode {
            PomodoroMode::Work => PomodoroMode::Break,
            PomodoroMode::Break => PomodoroMode::Work,
        };
        let next_minutes = match next_mode {
            PomodoroMode::Work => status.settings.default_work_minutes,
            PomodoroMode::Break => {
                if status.mode == PomodoroMode::Work
                    && status.completed_focus > 0
                    && status.completed_focus % status.settings.long_break_interval == 0
                {
                    status.settings.long_break_minutes
                } else {
                    status.settings.default_break_minutes
                }
            }
        };
        status
            .start(next_mode, next_minutes, now)
            .map_err(|err| err.to_string())?;
    }

    save_status(&conn, &status)?;
    drop(conn);

    if status.state == PomodoroState::Running {
        let trigger = match status.mode {
            PomodoroMode::Work => "pomodoro-started",
            PomodoroMode::Break => "pomodoro-resting",
        };
        mood_reactions.set(trigger, true);
    } else {
        mood_reactions.set("pomodoro-completed", false);
    }

    let (title, body) = match record.mode {
        PomodoroMode::Work => (
            "Focus Session Complete",
            "Great job! Time to take a short break.",
        ),
        PomodoroMode::Break => ("Break Complete", "Ready to start focusing again?"),
    };
    let message = if status.state == PomodoroState::Running {
        format!("{} {} session started automatically.", body, status.minutes)
    } else {
        body.to_string()
    };
    let _ = notifications.notify(Notification {
        source: POMODORO_OWNER.to_string(),
        title: title.to_string(),
        body: message,
        action_url: None,
        action_label: None,
        panel_label: None,
    });
    if status.state == PomodoroState::Running {
        // sync scheduler and badges for the new auto-started session
        let scheduler = Scheduler::new();
        scheduler.cancel(POMODORO_OWNER, POMODORO_TIMER_KEY);
        scheduler
            .set(
                POMODORO_OWNER,
                POMODORO_TIMER_KEY,
                status.time_remaining_secs,
                false,
                Some(status.time_remaining_secs),
            )
            .map_err(|err| err.to_string())?;

        let label = match (status.mode, status.state) {
            (PomodoroMode::Work, _) => "Focus".to_string(),
            (PomodoroMode::Break, _) => "Break".to_string(),
        };
        peek_badges.set(
            POMODORO_OWNER,
            vec![PeekBadgeItem {
                label,
                value: format_countdown(status.time_remaining_secs),
                icon: Some(match status.mode {
                    PomodoroMode::Work => "brain".to_string(),
                    PomodoroMode::Break => "coffee".to_string(),
                }),
                countdown_secs: Some(status.time_remaining_secs),
            }],
        );
    } else {
        peek_badges.clear(POMODORO_OWNER);
    }
    Ok(())
}

fn reconcile_after_wake(
    conn: &Arc<Mutex<Connection>>,
    peek_badges: &Arc<PeekBadgeService>,
) -> Result<(), String> {
    let conn = conn
        .lock()
        .map_err(|err| format!("Pomodoro db lock error: {err}"))?;
    let mut status = load_status(&conn)?;
    if status.state == PomodoroState::Running {
        status.refresh_time_remaining(now_epoch());
        save_status(&conn, &status)?;
    }

    if matches!(status.state, PomodoroState::Running | PomodoroState::Paused) {
        let label = match (status.mode, status.state) {
            (PomodoroMode::Work, PomodoroState::Paused) => "Focus (Paused)".to_string(),
            (PomodoroMode::Break, PomodoroState::Paused) => "Break (Paused)".to_string(),
            (PomodoroMode::Work, _) => "Focus".to_string(),
            (PomodoroMode::Break, _) => "Break".to_string(),
        };
        peek_badges.set(
            POMODORO_OWNER,
            vec![PeekBadgeItem {
                label,
                value: format_countdown(status.time_remaining_secs),
                icon: Some(match status.mode {
                    PomodoroMode::Work => "brain".to_string(),
                    PomodoroMode::Break => "coffee".to_string(),
                }),
                countdown_secs: (status.state == PomodoroState::Running)
                    .then_some(status.time_remaining_secs),
            }],
        );
    } else {
        peek_badges.clear(POMODORO_OWNER);
    }
    Ok(())
}

fn status_to_dto(status: &PomodoroStatus) -> PomodoroStatusDto {
    PomodoroStatusDto {
        mode: status.mode.as_str().to_string(),
        state: status.state.as_str().to_string(),
        minutes: status.minutes,
        time_remaining_secs: status.time_remaining_secs,
        completed_focus: status.completed_focus,
        completed_breaks: status.completed_breaks,
        enable_memo: status.settings.enable_memo,
        auto_advance: status.settings.auto_advance,
        default_work_minutes: status.settings.default_work_minutes,
        default_break_minutes: status.settings.default_break_minutes,
        long_break_minutes: status.settings.long_break_minutes,
        long_break_interval: status.settings.long_break_interval,
    }
}

fn parse_mode(input: &str) -> Result<PomodoroMode, String> {
    match input {
        "work" => Ok(PomodoroMode::Work),
        "break" => Ok(PomodoroMode::Break),
        other => Err(format!("invalid pomodoro mode: {other}")),
    }
}

fn parse_state(input: &str) -> Result<PomodoroState, String> {
    match input {
        "Idle" => Ok(PomodoroState::Idle),
        "Running" => Ok(PomodoroState::Running),
        "Paused" => Ok(PomodoroState::Paused),
        "Completed" => Ok(PomodoroState::Completed),
        other => Err(format!("invalid pomodoro state: {other}")),
    }
}

fn format_countdown(seconds: u64) -> String {
    let mins = seconds / 60;
    let secs = seconds % 60;
    format!("{:02}:{:02}", mins, secs)
}

fn now_epoch() -> i64 {
    Utc::now().timestamp()
}

fn iso_from_epoch(epoch: i64) -> Result<String, String> {
    let timestamp = DateTime::from_timestamp(epoch, 0)
        .ok_or_else(|| format!("Invalid pomodoro timestamp: {epoch}"))?;
    Ok(timestamp.to_rfc3339())
}

fn to_from_sql_error(message: String) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        Box::new(std::io::Error::other(message)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use peekoo_notifications::{MoodReactionService, NotificationService, PeekBadgeService};
    use peekoo_persistence_sqlite::{MIGRATION_0001_INIT, MIGRATION_0010_POMODORO_RUNTIME};

    fn create_service() -> PomodoroAppService {
        let conn = Arc::new(Mutex::new(
            Connection::open_in_memory().expect("in-memory db should open"),
        ));
        conn.lock()
            .expect("db lock")
            .execute_batch(MIGRATION_0001_INIT)
            .expect("base migration should apply");
        conn.lock()
            .expect("db lock")
            .execute_batch(MIGRATION_0010_POMODORO_RUNTIME)
            .expect("pomodoro migration should apply");
        conn.lock()
            .expect("db lock")
            .execute_batch(
                "ALTER TABLE pomodoro_state ADD COLUMN long_break_minutes INTEGER NOT NULL DEFAULT 15;
                 ALTER TABLE pomodoro_state ADD COLUMN long_break_interval INTEGER NOT NULL DEFAULT 4;
                 ALTER TABLE pomodoro_state ADD COLUMN auto_advance INTEGER NOT NULL DEFAULT 0;"
            )
            .expect("additional columns should be added");

        let (notifications, _receiver) = NotificationService::new();

        PomodoroAppService::new(
            conn,
            Arc::new(notifications),
            Arc::new(PeekBadgeService::new()),
            Arc::new(MoodReactionService::new()),
        )
        .expect("service should initialize")
    }

    #[test]
    fn get_status_returns_default_snapshot() {
        let service = create_service();

        let status = service.get_status().expect("status should load");

        assert_eq!(status.mode, "work");
        assert_eq!(status.state, "Idle");
        assert_eq!(status.default_work_minutes, 25);
        assert_eq!(status.default_break_minutes, 5);
        assert_eq!(status.completed_focus, 0);
        assert_eq!(status.completed_breaks, 0);
    }

    #[test]
    fn start_and_finish_persist_history() {
        let service = create_service();

        service.start("work", 25).expect("start should succeed");
        service.finish().expect("finish should succeed");

        let history = service.history(10).expect("history should load");

        assert_eq!(history.len(), 1);
        assert_eq!(history[0].mode, "work");
        assert_eq!(history[0].outcome, "cancelled");
    }

    #[test]
    fn set_settings_persists_enable_memo() {
        let service = create_service();

        let updated = service
            .set_settings(PomodoroSettingsInput {
                work_minutes: 40,
                break_minutes: 8,
                long_break_minutes: 15,
                long_break_interval: 4,
                enable_memo: true,
                auto_advance: true,
            })
            .expect("settings update should succeed");

        assert_eq!(updated.default_work_minutes, 40);
        assert_eq!(updated.default_break_minutes, 8);
        assert!(updated.enable_memo);
        assert!(updated.auto_advance);
    }
}
