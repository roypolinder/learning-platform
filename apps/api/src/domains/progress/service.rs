use std::collections::HashMap;
use std::sync::Arc;

use lp_core::{SessionStatus, StepStatus};
use lp_course_engine::view::{CourseSummaryView, course_summary};
use serde::Serialize;
use uuid::Uuid;

use crate::domains::courses::catalog::{LoadedCourse, RETIRED_VERSION};
use crate::domains::learners::repository as learners;
use crate::domains::sessions::dto::{SessionDto, percent, required_counts};
use crate::domains::sessions::repository as sessions;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

const RECENT_LIMIT: i64 = 12;

#[derive(Debug, Serialize)]
pub struct ProgressOverview {
    pub stats: ProgressStats,
    pub sessions: Vec<SessionSummary>,
}

#[derive(Debug, Default, Serialize)]
pub struct ProgressStats {
    pub active_courses: usize,
    pub completed_courses: usize,
    pub steps_completed: usize,
}

#[derive(Debug, Serialize)]
pub struct SessionSummary {
    pub session: SessionDto,
    pub course: CourseSummaryView,
    pub completed_required: usize,
    pub total_required: usize,
    pub percent: u8,
    pub current_step: Option<StepReference>,
}

#[derive(Debug, Serialize)]
pub struct StepReference {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CourseState {
    InProgress,
    Completed,
}

#[derive(Debug, Clone, Serialize)]
pub struct CourseProgress {
    pub state: CourseState,
    pub percent: u8,
    pub session_id: Uuid,
}

/// The learner's standing per course slug. A finished course stays `completed`
/// even if the learner later starts it again.
pub async fn course_progress(
    state: &AppState,
    user_id: Uuid,
) -> ApiResult<HashMap<String, CourseProgress>> {
    let all = sessions::list_for_user(&state.db, user_id, None, 500, 0).await?;
    let active: Vec<_> = all
        .iter()
        .filter(|session| session.status == SessionStatus::Active)
        .collect();
    let ids: Vec<Uuid> = active.iter().map(|session| session.id).collect();
    let mut statuses: HashMap<Uuid, HashMap<String, StepStatus>> = HashMap::new();
    for row in sessions::step_statuses_for_sessions(&state.db, &ids).await? {
        if let Ok(status) = row.status.parse() {
            statuses
                .entry(row.session_id)
                .or_default()
                .insert(row.step_id, status);
        }
    }

    let mut result = HashMap::new();
    // Sessions are newest first, so the first completed session per course wins.
    for session in all
        .iter()
        .filter(|session| session.status == SessionStatus::Completed)
    {
        result
            .entry(session.course_slug.clone())
            .or_insert(CourseProgress {
                state: CourseState::Completed,
                percent: 100,
                session_id: session.id,
            });
    }
    for session in active {
        if result.contains_key(&session.course_slug) {
            continue;
        }
        let Some(loaded) = loadable(state, session.course_version_id).await? else {
            continue;
        };
        let step_statuses = statuses.remove(&session.id).unwrap_or_default();
        let (completed, total) = required_counts(&loaded.course, |id| {
            step_statuses.get(id).copied().unwrap_or(StepStatus::Locked)
        });
        result.insert(
            session.course_slug.clone(),
            CourseProgress {
                state: CourseState::InProgress,
                percent: percent(completed, total),
                session_id: session.id,
            },
        );
    }
    Ok(result)
}

/// Course versions from before a breaking engine change no longer compile (the catalog
/// reports them once). Their sessions are left out of overviews instead of failing the page.
async fn loadable(state: &AppState, version_id: Uuid) -> ApiResult<Option<Arc<LoadedCourse>>> {
    match state
        .catalog
        .version(&state.db, &state.engine, version_id)
        .await
    {
        Ok(loaded) => Ok(Some(loaded)),
        Err(ApiError::Conflict {
            code: RETIRED_VERSION,
            ..
        }) => Ok(None),
        Err(error) => Err(error),
    }
}

pub async fn overview(state: &AppState, user_id: Uuid) -> ApiResult<ProgressOverview> {
    let recent = sessions::list_for_user(&state.db, user_id, None, RECENT_LIMIT, 0).await?;
    let ids: Vec<Uuid> = recent.iter().map(|session| session.id).collect();
    let mut statuses: HashMap<Uuid, HashMap<String, StepStatus>> = HashMap::new();
    for row in sessions::step_statuses_for_sessions(&state.db, &ids).await? {
        if let Ok(status) = row.status.parse() {
            statuses
                .entry(row.session_id)
                .or_default()
                .insert(row.step_id, status);
        }
    }

    let profile = learners::profile(&state.db, user_id).await?;
    let mut stats = ProgressStats::default();
    let mut summaries = Vec::with_capacity(recent.len());
    for session in &recent {
        let Some(loaded) = loadable(state, session.course_version_id).await? else {
            continue;
        };
        let loaded =
            state
                .catalog
                .render(&state.engine, &loaded, profile.audience(&state.themes))?;
        let course = &loaded.course;
        let step_statuses = statuses.remove(&session.id).unwrap_or_default();
        let status_of = |id: &str| step_statuses.get(id).copied().unwrap_or(StepStatus::Locked);
        let (completed_required, total_required) = required_counts(course, status_of);
        stats.steps_completed += step_statuses
            .values()
            .filter(|s| **s == StepStatus::Completed)
            .count();
        match session.status {
            SessionStatus::Active => stats.active_courses += 1,
            SessionStatus::Completed => stats.completed_courses += 1,
            SessionStatus::Abandoned => {}
        }
        let current_step = session
            .current_step_id
            .as_deref()
            .and_then(|id| course.step(id))
            .map(|step| StepReference {
                id: step.id.clone(),
                title: step.title.clone(),
            });
        summaries.push(SessionSummary {
            session: SessionDto::from(session),
            course: course_summary(course),
            completed_required,
            total_required,
            percent: percent(completed_required, total_required),
            current_step,
        });
    }
    Ok(ProgressOverview {
        stats,
        sessions: summaries,
    })
}
