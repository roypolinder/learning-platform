use chrono::{DateTime, Utc};
use lp_core::SessionStatus;
use lp_course_engine::StepState;
use serde_json::Value;
use sqlx::{PgConnection, PgExecutor};
use uuid::Uuid;

use super::domain::{CourseSession, NewEvent, SessionEvent, StepRow};

/// Builds a `'static` session query; SQL is never assembled at runtime.
macro_rules! select_sessions {
    ($filter:literal $(,)?) => {
        concat!(
            "SELECT s.id, s.user_id, s.course_id, c.slug AS course_slug, s.course_version_id, s.status, ",
            "s.current_step_id, s.started_at, s.updated_at, s.last_accessed_at, s.completed_at ",
            "FROM course_sessions s JOIN courses c ON c.id = s.course_id ",
            $filter
        )
    };
}

/// Returns `false` when the learner already has an active session for the course.
pub async fn insert(
    connection: &mut PgConnection,
    id: Uuid,
    user_id: Uuid,
    course_id: Uuid,
    course_version_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO course_sessions (id, user_id, course_id, course_version_id, status)
         VALUES ($1, $2, $3, $4, 'active')
         ON CONFLICT (user_id, course_id) WHERE status = 'active' DO NOTHING",
    )
    .bind(id)
    .bind(user_id)
    .bind(course_id)
    .bind(course_version_id)
    .execute(connection)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn find_for_user(
    executor: impl PgExecutor<'_>,
    id: Uuid,
    user_id: Uuid,
) -> Result<Option<CourseSession>, sqlx::Error> {
    sqlx::query_as(select_sessions!("WHERE s.id = $1 AND s.user_id = $2"))
        .bind(id)
        .bind(user_id)
        .fetch_optional(executor)
        .await
}

/// Row lock serializing all state changes of one session.
pub async fn lock_for_user(
    connection: &mut PgConnection,
    id: Uuid,
    user_id: Uuid,
) -> Result<Option<CourseSession>, sqlx::Error> {
    sqlx::query_as(select_sessions!(
        "WHERE s.id = $1 AND s.user_id = $2 FOR UPDATE OF s"
    ))
    .bind(id)
    .bind(user_id)
    .fetch_optional(connection)
    .await
}

pub async fn find_active(
    executor: impl PgExecutor<'_>,
    user_id: Uuid,
    course_id: Uuid,
) -> Result<Option<CourseSession>, sqlx::Error> {
    sqlx::query_as(select_sessions!(
        "WHERE s.user_id = $1 AND s.course_id = $2 AND s.status = 'active'",
    ))
    .bind(user_id)
    .bind(course_id)
    .fetch_optional(executor)
    .await
}

pub async fn find_latest_for_course(
    executor: impl PgExecutor<'_>,
    user_id: Uuid,
    course_id: Uuid,
) -> Result<Option<CourseSession>, sqlx::Error> {
    sqlx::query_as(select_sessions!(
        "WHERE s.user_id = $1 AND s.course_id = $2
         ORDER BY (s.status = 'active') DESC, s.last_accessed_at DESC LIMIT 1",
    ))
    .bind(user_id)
    .bind(course_id)
    .fetch_optional(executor)
    .await
}

pub async fn most_recent_active(
    executor: impl PgExecutor<'_>,
    user_id: Uuid,
) -> Result<Option<CourseSession>, sqlx::Error> {
    sqlx::query_as(select_sessions!(
        "WHERE s.user_id = $1 AND s.status = 'active' ORDER BY s.last_accessed_at DESC LIMIT 1",
    ))
    .bind(user_id)
    .fetch_optional(executor)
    .await
}

pub async fn list_for_user(
    executor: impl PgExecutor<'_>,
    user_id: Uuid,
    status: Option<SessionStatus>,
    limit: i64,
    offset: i64,
) -> Result<Vec<CourseSession>, sqlx::Error> {
    sqlx::query_as(select_sessions!(
        "WHERE s.user_id = $1 AND ($2::text IS NULL OR s.status = $2)
         ORDER BY s.last_accessed_at DESC LIMIT $3 OFFSET $4",
    ))
    .bind(user_id)
    .bind(status.map(SessionStatus::as_str))
    .bind(limit)
    .bind(offset)
    .fetch_all(executor)
    .await
}

pub async fn count_for_user(
    executor: impl PgExecutor<'_>,
    user_id: Uuid,
    status: Option<SessionStatus>,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT count(*) FROM course_sessions WHERE user_id = $1 AND ($2::text IS NULL OR status = $2)",
    )
    .bind(user_id)
    .bind(status.map(SessionStatus::as_str))
    .fetch_one(executor)
    .await
}

pub async fn touch(executor: impl PgExecutor<'_>, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE course_sessions SET last_accessed_at = now() WHERE id = $1")
        .bind(id)
        .execute(executor)
        .await?;
    Ok(())
}

pub async fn update_state(
    connection: &mut PgConnection,
    id: Uuid,
    status: SessionStatus,
    current_step_id: Option<&str>,
    completed_at: Option<DateTime<Utc>>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE course_sessions
         SET status = $2, current_step_id = $3, completed_at = $4, updated_at = now(), last_accessed_at = now()
         WHERE id = $1",
    )
    .bind(id)
    .bind(status.as_str())
    .bind(current_step_id)
    .bind(completed_at)
    .execute(connection)
    .await?;
    Ok(())
}

pub async fn load_steps(
    executor: impl PgExecutor<'_>,
    session_id: Uuid,
) -> Result<Vec<StepRow>, sqlx::Error> {
    sqlx::query_as(
        "SELECT step_id, status, attempts, score, last_result FROM step_progress WHERE session_id = $1",
    )
    .bind(session_id)
    .fetch_all(executor)
    .await
}

pub async fn insert_steps(
    connection: &mut PgConnection,
    session_id: Uuid,
    steps: &[(&str, &StepState)],
) -> Result<(), sqlx::Error> {
    if steps.is_empty() {
        return Ok(());
    }
    let ids: Vec<Uuid> = steps.iter().map(|_| Uuid::now_v7()).collect();
    let step_ids: Vec<&str> = steps.iter().map(|(id, _)| *id).collect();
    let statuses: Vec<&str> = steps
        .iter()
        .map(|(_, state)| state.status.as_str())
        .collect();
    sqlx::query(
        "INSERT INTO step_progress (id, session_id, step_id, status, completed_at)
         SELECT id, $2, step_id, status, CASE WHEN status IN ('completed', 'skipped') THEN now() END
         FROM UNNEST($1::uuid[], $3::text[], $4::text[]) AS t(id, step_id, status)",
    )
    .bind(&ids)
    .bind(session_id)
    .bind(&step_ids)
    .bind(&statuses)
    .execute(connection)
    .await?;
    Ok(())
}

pub async fn update_step(
    connection: &mut PgConnection,
    session_id: Uuid,
    step_id: &str,
    state: &StepState,
) -> Result<(), sqlx::Error> {
    let last_result = state
        .last_outcome
        .as_ref()
        .and_then(|outcome| serde_json::to_value(outcome).ok());
    sqlx::query(
        "UPDATE step_progress SET
           status = $3,
           attempts = $4,
           score = $5,
           last_result = $6,
           started_at = COALESCE(started_at, CASE WHEN $3 <> 'locked' AND $3 <> 'available' THEN now() END),
           completed_at = CASE WHEN $3 IN ('completed', 'skipped') THEN COALESCE(completed_at, now()) END,
           updated_at = now()
         WHERE session_id = $1 AND step_id = $2",
    )
    .bind(session_id)
    .bind(step_id)
    .bind(state.status.as_str())
    .bind(i32::try_from(state.attempts).unwrap_or(i32::MAX))
    .bind(state.score)
    .bind(last_result)
    .execute(connection)
    .await?;
    Ok(())
}

pub async fn insert_events(
    connection: &mut PgConnection,
    session_id: Uuid,
    events: &[NewEvent],
) -> Result<(), sqlx::Error> {
    if events.is_empty() {
        return Ok(());
    }
    let ids: Vec<Uuid> = events.iter().map(|_| Uuid::now_v7()).collect();
    let step_ids: Vec<Option<&str>> = events
        .iter()
        .map(|event| event.step_id.as_deref())
        .collect();
    let kinds: Vec<&str> = events.iter().map(|event| event.kind).collect();
    let payloads: Vec<Value> = events.iter().map(|event| event.payload.clone()).collect();
    sqlx::query(
        "INSERT INTO session_events (id, session_id, step_id, kind, payload)
         SELECT id, $2, step_id, kind, payload
         FROM UNNEST($1::uuid[], $3::text[], $4::text[], $5::jsonb[]) AS t(id, step_id, kind, payload)",
    )
    .bind(&ids)
    .bind(session_id)
    .bind(&step_ids)
    .bind(&kinds)
    .bind(&payloads)
    .execute(connection)
    .await?;
    Ok(())
}

pub async fn list_events(
    executor: impl PgExecutor<'_>,
    session_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<SessionEvent>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, step_id, kind, payload, created_at FROM session_events
         WHERE session_id = $1 ORDER BY created_at DESC, id DESC LIMIT $2 OFFSET $3",
    )
    .bind(session_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(executor)
    .await
}

pub async fn completed_course_slugs(
    executor: impl PgExecutor<'_>,
    user_id: Uuid,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT DISTINCT c.slug FROM course_sessions s JOIN courses c ON c.id = s.course_id
         WHERE s.user_id = $1 AND s.status = 'completed'",
    )
    .bind(user_id)
    .fetch_all(executor)
    .await
}

#[derive(Debug, sqlx::FromRow)]
pub struct SessionStepStatus {
    pub session_id: Uuid,
    pub step_id: String,
    pub status: String,
}

pub async fn step_statuses_for_sessions(
    executor: impl PgExecutor<'_>,
    session_ids: &[Uuid],
) -> Result<Vec<SessionStepStatus>, sqlx::Error> {
    sqlx::query_as(
        "SELECT session_id, step_id, status FROM step_progress WHERE session_id = ANY($1)",
    )
    .bind(session_ids)
    .fetch_all(executor)
    .await
}
