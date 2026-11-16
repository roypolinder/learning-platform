use chrono::{DateTime, Utc};
use lp_core::{SessionStatus, StepStatus, ValidationOutcome};
use lp_course_engine::view::CourseDetailView;
use lp_course_engine::{Course, CourseEngine, Progress};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::domain::{CourseSession, SessionEvent};

#[derive(Debug, Serialize)]
pub struct SessionDto {
    pub id: Uuid,
    pub course_id: String,
    pub course_version_id: Uuid,
    pub status: SessionStatus,
    pub current_step_id: Option<String>,
    pub started_at: DateTime<Utc>,
    pub last_accessed_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl From<&CourseSession> for SessionDto {
    fn from(session: &CourseSession) -> Self {
        Self {
            id: session.id,
            course_id: session.course_slug.clone(),
            course_version_id: session.course_version_id,
            status: session.status,
            current_step_id: session.current_step_id.clone(),
            started_at: session.started_at,
            last_accessed_at: session.last_accessed_at,
            completed_at: session.completed_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct StepProgressDto {
    pub step_id: String,
    pub status: StepStatus,
    pub attempts: u32,
    pub score: Option<f64>,
    pub outcome: Option<ValidationOutcome>,
}

#[derive(Debug, Serialize)]
pub struct ProgressDto {
    pub session_id: Uuid,
    pub status: SessionStatus,
    pub current_step_id: Option<String>,
    pub completed_required: usize,
    pub total_required: usize,
    pub percent: u8,
    pub steps: Vec<StepProgressDto>,
}

impl ProgressDto {
    pub fn build(
        engine: &CourseEngine,
        session: &CourseSession,
        course: &Course,
        progress: &Progress,
    ) -> Self {
        let (completed_required, total_required) =
            required_counts(course, |id| progress.status(id));
        Self {
            session_id: session.id,
            status: session.status,
            current_step_id: engine
                .current_step(course, progress)
                .map(|step| step.id.clone()),
            completed_required,
            total_required,
            percent: percent(completed_required, total_required),
            steps: course
                .steps
                .iter()
                .map(|step| {
                    let state = progress.state(&step.id);
                    StepProgressDto {
                        step_id: step.id.clone(),
                        status: progress.status(&step.id),
                        attempts: state.map_or(0, |state| state.attempts),
                        score: state.and_then(|state| state.score),
                        outcome: state.and_then(|state| state.last_outcome.clone()),
                    }
                })
                .collect(),
        }
    }
}

pub fn required_counts(course: &Course, status: impl Fn(&str) -> StepStatus) -> (usize, usize) {
    let completed = course
        .required_steps
        .iter()
        .filter(|id| status(id).is_resolved())
        .count();
    (completed, course.required_steps.len())
}

pub fn percent(completed: usize, total: usize) -> u8 {
    (completed * 100)
        .checked_div(total)
        .map_or(100, |value| u8::try_from(value).unwrap_or(100))
}

#[derive(Debug, Serialize)]
pub struct SessionDetailResponse {
    pub session: SessionDto,
    pub course: CourseDetailView,
    pub progress: ProgressDto,
}

#[derive(Debug, Serialize)]
pub struct StepActionResponse {
    pub progress: ProgressDto,
    /// Result of the validation triggered by this request, if any.
    pub outcome: Option<ValidationOutcome>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubmissionRequest {
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Default, Deserialize)]
pub struct SessionListQuery {
    pub status: Option<SessionStatus>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct SessionEventDto {
    pub id: Uuid,
    pub step_id: Option<String>,
    pub kind: String,
    pub payload: Value,
    pub created_at: DateTime<Utc>,
}

impl From<SessionEvent> for SessionEventDto {
    fn from(event: SessionEvent) -> Self {
        Self {
            id: event.id,
            step_id: event.step_id,
            kind: event.kind,
            payload: event.payload.0,
            created_at: event.created_at,
        }
    }
}
