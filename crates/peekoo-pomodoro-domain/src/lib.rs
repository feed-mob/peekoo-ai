use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PomodoroMode {
    Work,
    Break,
}

impl PomodoroMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Work => "work",
            Self::Break => "break",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PomodoroState {
    Idle,
    Running,
    Paused,
    Completed,
}

impl PomodoroState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::Running => "Running",
            Self::Paused => "Paused",
            Self::Completed => "Completed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PomodoroCycleOutcome {
    Completed,
    Cancelled,
}

impl PomodoroCycleOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PomodoroSettings {
    pub default_work_minutes: u32,
    pub default_break_minutes: u32,
    pub long_break_minutes: u32,
    pub long_break_interval: u32,
    pub enable_memo: bool,
    pub auto_advance: bool,
}

impl PomodoroSettings {
    pub fn new(
        default_work_minutes: u32,
        default_break_minutes: u32,
        long_break_minutes: u32,
        long_break_interval: u32,
        enable_memo: bool,
        auto_advance: bool,
    ) -> Result<Self, PomodoroError> {
        if default_work_minutes == 0 {
            return Err(PomodoroError::InvalidWorkMinutes);
        }
        if default_break_minutes == 0 {
            return Err(PomodoroError::InvalidBreakMinutes);
        }
        if long_break_interval == 0 {
            return Err(PomodoroError::InvalidLongBreakInterval);
        }

        Ok(Self {
            default_work_minutes,
            default_break_minutes,
            long_break_minutes,
            long_break_interval,
            enable_memo,
            auto_advance,
        })
    }

    pub fn minutes_for_mode(&self, mode: PomodoroMode) -> u32 {
        match mode {
            PomodoroMode::Work => self.default_work_minutes,
            PomodoroMode::Break => self.default_break_minutes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PomodoroCycleRecord {
    pub mode: PomodoroMode,
    pub planned_minutes: u32,
    pub actual_elapsed_secs: u64,
    pub outcome: PomodoroCycleOutcome,
    pub started_at_epoch: i64,
    pub ended_at_epoch: i64,
    pub memo_requested: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PomodoroStatus {
    pub mode: PomodoroMode,
    pub state: PomodoroState,
    pub minutes: u32,
    pub time_remaining_secs: u64,
    pub started_at_epoch: Option<i64>,
    pub expected_fire_at_epoch: Option<i64>,
    pub completed_focus: u32,
    pub completed_breaks: u32,
    pub last_reset_date: Option<String>,
    pub settings: PomodoroSettings,
}

impl PomodoroStatus {
    pub fn new(settings: PomodoroSettings) -> Self {
        let minutes = settings.default_work_minutes;
        Self {
            mode: PomodoroMode::Work,
            state: PomodoroState::Idle,
            minutes,
            time_remaining_secs: u64::from(minutes) * 60,
            started_at_epoch: None,
            expected_fire_at_epoch: None,
            completed_focus: 0,
            completed_breaks: 0,
            last_reset_date: None,
            settings,
        }
    }

    pub fn set_settings(&mut self, settings: PomodoroSettings) {
        self.settings = settings;
        if matches!(self.state, PomodoroState::Idle | PomodoroState::Completed) {
            self.minutes = self.settings.minutes_for_mode(self.mode);
            self.time_remaining_secs = u64::from(self.minutes) * 60;
        }
    }

    pub fn start(
        &mut self,
        mode: PomodoroMode,
        minutes: u32,
        now_epoch: i64,
    ) -> Result<(), PomodoroError> {
        if minutes == 0 {
            return Err(PomodoroError::InvalidSessionMinutes);
        }
        if matches!(self.state, PomodoroState::Running | PomodoroState::Paused) {
            return Err(PomodoroError::AlreadyActive);
        }

        self.mode = mode;
        self.state = PomodoroState::Running;
        self.minutes = minutes;
        self.time_remaining_secs = u64::from(minutes) * 60;
        self.started_at_epoch = Some(now_epoch);
        self.expected_fire_at_epoch = Some(now_epoch + i64::from(minutes) * 60);
        Ok(())
    }

    pub fn pause(&mut self, now_epoch: i64) -> Result<(), PomodoroError> {
        if self.state != PomodoroState::Running {
            return Err(PomodoroError::InvalidPause);
        }

        self.refresh_time_remaining(now_epoch);
        self.state = PomodoroState::Paused;
        self.expected_fire_at_epoch = None;
        Ok(())
    }

    pub fn resume(&mut self, now_epoch: i64) -> Result<(), PomodoroError> {
        if self.state != PomodoroState::Paused {
            return Err(PomodoroError::InvalidResume);
        }

        self.state = PomodoroState::Running;
        self.expected_fire_at_epoch = Some(now_epoch + self.time_remaining_secs as i64);
        Ok(())
    }

    pub fn finish(&mut self, now_epoch: i64) -> Result<PomodoroCycleRecord, PomodoroError> {
        if !matches!(self.state, PomodoroState::Running | PomodoroState::Paused) {
            return Err(PomodoroError::InvalidFinish);
        }

        if self.state == PomodoroState::Running {
            self.refresh_time_remaining(now_epoch);
        }

        let record = self.build_record(now_epoch, PomodoroCycleOutcome::Cancelled)?;
        self.reset_to_idle();
        Ok(record)
    }

    pub fn complete(&mut self, now_epoch: i64) -> Result<PomodoroCycleRecord, PomodoroError> {
        if self.state != PomodoroState::Running {
            return Err(PomodoroError::InvalidCompletion);
        }

        self.time_remaining_secs = 0;
        self.expected_fire_at_epoch = Some(now_epoch);

        match self.mode {
            PomodoroMode::Work => self.completed_focus += 1,
            PomodoroMode::Break => self.completed_breaks += 1,
        }

        let record = self.build_record(now_epoch, PomodoroCycleOutcome::Completed)?;
        self.state = PomodoroState::Completed;
        self.expected_fire_at_epoch = None;
        Ok(record)
    }

    pub fn switch_mode(&mut self, mode: PomodoroMode) -> Result<(), PomodoroError> {
        if matches!(self.state, PomodoroState::Running | PomodoroState::Paused) {
            return Err(PomodoroError::CannotSwitchWhileActive);
        }

        self.mode = mode;
        self.reset_to_idle();
        Ok(())
    }

    pub fn refresh_time_remaining(&mut self, now_epoch: i64) {
        if self.state != PomodoroState::Running {
            return;
        }

        let Some(expected_fire_at_epoch) = self.expected_fire_at_epoch else {
            return;
        };

        if now_epoch >= expected_fire_at_epoch {
            self.time_remaining_secs = 0;
        } else {
            self.time_remaining_secs = (expected_fire_at_epoch - now_epoch) as u64;
        }
    }

    fn build_record(
        &self,
        now_epoch: i64,
        outcome: PomodoroCycleOutcome,
    ) -> Result<PomodoroCycleRecord, PomodoroError> {
        let started_at_epoch = self
            .started_at_epoch
            .ok_or(PomodoroError::MissingStartTime)?;
        let planned_seconds = u64::from(self.minutes) * 60;
        let actual_elapsed_secs = planned_seconds.saturating_sub(self.time_remaining_secs);

        Ok(PomodoroCycleRecord {
            mode: self.mode,
            planned_minutes: self.minutes,
            actual_elapsed_secs,
            outcome,
            started_at_epoch,
            ended_at_epoch: now_epoch,
            memo_requested: self.mode == PomodoroMode::Work
                && outcome == PomodoroCycleOutcome::Completed
                && self.settings.enable_memo,
        })
    }

    fn reset_to_idle(&mut self) {
        self.state = PomodoroState::Idle;
        self.started_at_epoch = None;
        self.expected_fire_at_epoch = None;
        self.minutes = self.settings.minutes_for_mode(self.mode);
        self.time_remaining_secs = u64::from(self.minutes) * 60;
    }
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum PomodoroError {
    #[error("work minutes must be greater than zero")]
    InvalidWorkMinutes,
    #[error("break minutes must be greater than zero")]
    InvalidBreakMinutes,
    #[error("long break interval must be greater than zero")]
    InvalidLongBreakInterval,
    #[error("session minutes must be greater than zero")]
    InvalidSessionMinutes,
    #[error("pomodoro is already active")]
    AlreadyActive,
    #[error("cannot pause unless running")]
    InvalidPause,
    #[error("cannot resume unless paused")]
    InvalidResume,
    #[error("cannot finish unless running or paused")]
    InvalidFinish,
    #[error("cannot complete unless running")]
    InvalidCompletion,
    #[error("cannot switch mode while running or paused")]
    CannotSwitchWhileActive,
    #[error("missing pomodoro start time")]
    MissingStartTime,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_status_uses_default_work_settings() {
        let settings =
            PomodoroSettings::new(25, 5, 15, 4, true, false).expect("settings should be valid");
        let status = PomodoroStatus::new(settings.clone());

        assert_eq!(status.mode, PomodoroMode::Work);
        assert_eq!(status.state, PomodoroState::Idle);
        assert_eq!(status.minutes, 25);
        assert_eq!(status.time_remaining_secs, 25 * 60);
        assert_eq!(status.settings, settings);
        assert_eq!(status.completed_focus, 0);
        assert_eq!(status.completed_breaks, 0);
        assert_eq!(status.last_reset_date, None);
    }

    #[test]
    fn completing_work_session_updates_focus_counter() {
        let settings =
            PomodoroSettings::new(25, 5, 15, 4, true, false).expect("settings should be valid");
        let mut status = PomodoroStatus::new(settings);

        status
            .start(PomodoroMode::Work, 25, 1_000)
            .expect("start should succeed");
        let record = status.complete(2_500).expect("completion should succeed");

        assert_eq!(status.state, PomodoroState::Completed);
        assert_eq!(status.time_remaining_secs, 0);
        assert_eq!(status.completed_focus, 1);
        assert_eq!(status.completed_breaks, 0);
        assert_eq!(record.mode, PomodoroMode::Work);
        assert_eq!(record.outcome, PomodoroCycleOutcome::Completed);
        assert!(record.memo_requested);
    }

    #[test]
    fn cancelling_break_session_records_history_without_incrementing_focus() {
        let settings =
            PomodoroSettings::new(25, 5, 15, 4, false, false).expect("settings should be valid");
        let mut status = PomodoroStatus::new(settings);

        status
            .start(PomodoroMode::Break, 5, 1_000)
            .expect("start should succeed");
        let record = status.finish(1_120).expect("finish should succeed");

        assert_eq!(status.state, PomodoroState::Idle);
        assert_eq!(status.completed_focus, 0);
        assert_eq!(status.completed_breaks, 0);
        assert_eq!(record.mode, PomodoroMode::Break);
        assert_eq!(record.outcome, PomodoroCycleOutcome::Cancelled);
        assert_eq!(record.actual_elapsed_secs, 120);
    }

    #[test]
    fn switching_mode_while_idle_resets_duration_from_settings() {
        let settings =
            PomodoroSettings::new(30, 7, 20, 4, false, false).expect("settings should be valid");
        let mut status = PomodoroStatus::new(settings);

        status
            .switch_mode(PomodoroMode::Break)
            .expect("switch should succeed");

        assert_eq!(status.mode, PomodoroMode::Break);
        assert_eq!(status.minutes, 7);
        assert_eq!(status.time_remaining_secs, 7 * 60);
    }

    #[test]
    fn invalid_settings_are_rejected() {
        let result = PomodoroSettings::new(0, 5, 15, 4, true, false);

        assert_eq!(result.unwrap_err(), PomodoroError::InvalidWorkMinutes);
    }
}
