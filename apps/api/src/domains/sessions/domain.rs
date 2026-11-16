use chrono::{DateTime, Utc};
use lp_core::{SessionStatus, StepStatus, ValidationOutcome};
use lp_course_engine::{Progress, StepState};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CourseSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub course_id: Uuid,
    pub course_slug: String,
    pub course_version_id: Uuid,
    #[sqlx(try_from = "String")]
    pub status: SessionStatus,
    pub current_step_id: Option<String>,
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_accessed_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StepRow {
    pub step_id: String,
    #[sqlx(try_from = "String")]
    pub status: StepStatus,
    pub attempts: i32,
    pub score: Option<f64>,
    pub last_result: Option<sqlx::types::Json<Value>>,
}

impl StepRow {
    pub fn into_state(self) -> (String, StepState) {
        let last_outcome = self
            .last_result
            .and_then(|json| serde_json::from_value::<ValidationOutcome>(json.0).ok());
        (
            self.step_id,
            StepState {
                status: self.status,
                attempts: u32::try_from(self.attempts).unwrap_or_default(),
                score: self.score,
                last_outcome,
            },
        )
    }
}

pub fn progress_from_rows(rows: Vec<StepRow>) -> Progress {
    Progress {
        steps: rows.into_iter().map(StepRow::into_state).collect(),
    }
}

#[derive(Debug, Clone)]
pub struct NewEvent {
    pub step_id: Option<String>,
    pub kind: &'static str,
    pub payload: Value,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SessionEvent {
    pub id: Uuid,
    pub step_id: Option<String>,
    pub kind: String,
    pub payload: sqlx::types::Json<Value>,
    pub created_at: DateTime<Utc>,
}

/// Who caused a state change; recorded in the audit trail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Actor {
    Learner,
    Agent,
}

impl Actor {
    pub fn as_str(self) -> &'static str {
        match self {
            Actor::Learner => "learner",
            Actor::Agent => "agent",
        }
    }
}
