use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PomodoroState {
    Idle,
    Running,
    Paused,
    Completed,
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum PomodoroError {
    #[error("cannot start when state is not idle")]
    InvalidStart,
    #[error("cannot pause unless running")]
    InvalidPause,
    #[error("cannot resume unless paused")]
    InvalidResume,
    #[error("cannot finish unless running or paused")]
    InvalidFinish,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PomodoroSession {
    pub id: String,
    pub minutes: u32,
    pub state: PomodoroState,
}

impl PomodoroSession {
    pub fn new(id: impl Into<String>, minutes: u32) -> Self {
        Self {
            id: id.into(),
            minutes,
            state: PomodoroState::Idle,
        }
    }

    pub fn start(&mut self) -> Result<(), PomodoroError> {
        if self.state != PomodoroState::Idle {
            return Err(PomodoroError::InvalidStart);
        }
        self.state = PomodoroState::Running;
        Ok(())
    }

    pub fn pause(&mut self) -> Result<(), PomodoroError> {
        if self.state != PomodoroState::Running {
            return Err(PomodoroError::InvalidPause);
        }
        self.state = PomodoroState::Paused;
        Ok(())
    }

    pub fn resume(&mut self) -> Result<(), PomodoroError> {
        if self.state != PomodoroState::Paused {
            return Err(PomodoroError::InvalidResume);
        }
        self.state = PomodoroState::Running;
        Ok(())
    }

    pub fn finish(&mut self) -> Result<(), PomodoroError> {
        if self.state != PomodoroState::Running && self.state != PomodoroState::Paused {
            return Err(PomodoroError::InvalidFinish);
        }
        self.state = PomodoroState::Completed;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_state_transition_for_pomodoro() {
        let mut session = PomodoroSession::new("pom-1", 25);
        assert_eq!(session.state, PomodoroState::Idle);
        assert_eq!(session.start(), Ok(()));
        assert_eq!(session.pause(), Ok(()));
        assert_eq!(session.resume(), Ok(()));
        assert_eq!(session.finish(), Ok(()));
        assert_eq!(session.state, PomodoroState::Completed);
    }

    #[test]
    fn invalid_pause_from_idle_is_rejected() {
        let mut session = PomodoroSession::new("pom-1", 25);
        assert_eq!(session.pause(), Err(PomodoroError::InvalidPause));
    }
}
