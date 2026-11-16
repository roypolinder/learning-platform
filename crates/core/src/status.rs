use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    #[default]
    Locked,
    Available,
    InProgress,
    Completed,
    Failed,
    Skipped,
}

impl StepStatus {
    pub const ALL: [StepStatus; 6] = [
        StepStatus::Locked,
        StepStatus::Available,
        StepStatus::InProgress,
        StepStatus::Completed,
        StepStatus::Failed,
        StepStatus::Skipped,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            StepStatus::Locked => "locked",
            StepStatus::Available => "available",
            StepStatus::InProgress => "in_progress",
            StepStatus::Completed => "completed",
            StepStatus::Failed => "failed",
            StepStatus::Skipped => "skipped",
        }
    }

    /// Resolved steps satisfy dependencies of later steps.
    pub fn is_resolved(self) -> bool {
        matches!(self, StepStatus::Completed | StepStatus::Skipped)
    }

    /// Steps a learner can currently interact with.
    pub fn is_actionable(self) -> bool {
        matches!(
            self,
            StepStatus::Available | StepStatus::InProgress | StepStatus::Failed
        )
    }

    pub fn can_transition_to(self, next: StepStatus) -> bool {
        use StepStatus::*;
        matches!(
            (self, next),
            (Locked, Available | Skipped)
                | (Available, InProgress | Completed | Failed | Skipped)
                | (InProgress, Completed | Failed | Skipped)
                | (Failed, InProgress | Completed)
        )
    }

    pub fn transition_to(self, next: StepStatus) -> Result<StepStatus, TransitionError> {
        if self.can_transition_to(next) {
            Ok(next)
        } else {
            Err(TransitionError::Step {
                from: self,
                to: next,
            })
        }
    }
}

impl fmt::Display for StepStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for StepStatus {
    type Err = UnknownStatus;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        StepStatus::ALL
            .into_iter()
            .find(|status| status.as_str() == value)
            .ok_or_else(|| UnknownStatus(value.to_owned()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Completed,
    Abandoned,
}

impl SessionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            SessionStatus::Active => "active",
            SessionStatus::Completed => "completed",
            SessionStatus::Abandoned => "abandoned",
        }
    }

    pub fn can_transition_to(self, next: SessionStatus) -> bool {
        use SessionStatus::*;
        matches!(
            (self, next),
            (Active, Completed | Abandoned) | (Abandoned, Active)
        )
    }

    pub fn transition_to(self, next: SessionStatus) -> Result<SessionStatus, TransitionError> {
        if self.can_transition_to(next) {
            Ok(next)
        } else {
            Err(TransitionError::Session {
                from: self,
                to: next,
            })
        }
    }
}

impl fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for SessionStatus {
    type Err = UnknownStatus;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "active" => Ok(SessionStatus::Active),
            "completed" => Ok(SessionStatus::Completed),
            "abandoned" => Ok(SessionStatus::Abandoned),
            other => Err(UnknownStatus(other.to_owned())),
        }
    }
}

impl TryFrom<String> for StepStatus {
    type Error = UnknownStatus;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl TryFrom<String> for SessionStatus {
    type Error = UnknownStatus;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

#[derive(Debug, thiserror::Error)]
#[error("unknown status `{0}`")]
pub struct UnknownStatus(pub String);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TransitionError {
    #[error("step cannot move from {from} to {to}")]
    Step { from: StepStatus, to: StepStatus },
    #[error("session cannot move from {from} to {to}")]
    Session {
        from: SessionStatus,
        to: SessionStatus,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locked_steps_only_unlock_or_skip() {
        assert!(StepStatus::Locked.can_transition_to(StepStatus::Available));
        assert!(StepStatus::Locked.can_transition_to(StepStatus::Skipped));
        assert!(!StepStatus::Locked.can_transition_to(StepStatus::Completed));
        assert!(!StepStatus::Locked.can_transition_to(StepStatus::InProgress));
    }

    #[test]
    fn resolved_steps_are_final() {
        for next in StepStatus::ALL {
            assert!(!StepStatus::Completed.can_transition_to(next));
            assert!(!StepStatus::Skipped.can_transition_to(next));
        }
    }

    #[test]
    fn failed_steps_can_be_retried() {
        assert!(StepStatus::Failed.can_transition_to(StepStatus::InProgress));
        assert!(StepStatus::Failed.can_transition_to(StepStatus::Completed));
        assert_eq!(
            StepStatus::Failed.transition_to(StepStatus::Locked),
            Err(TransitionError::Step {
                from: StepStatus::Failed,
                to: StepStatus::Locked
            })
        );
    }

    #[test]
    fn status_round_trips_through_strings() {
        for status in StepStatus::ALL {
            assert_eq!(status.as_str().parse::<StepStatus>().unwrap(), status);
        }
        assert!("done".parse::<StepStatus>().is_err());
    }

    #[test]
    fn completed_sessions_cannot_be_reopened() {
        assert!(!SessionStatus::Completed.can_transition_to(SessionStatus::Active));
        assert!(SessionStatus::Abandoned.can_transition_to(SessionStatus::Active));
    }
}
