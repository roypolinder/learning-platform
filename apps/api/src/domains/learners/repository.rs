use chrono::{DateTime, Utc};
use sqlx::PgExecutor;
use uuid::Uuid;

use super::domain::{LearnerProfile, Project, Stack};

#[derive(sqlx::FromRow)]
struct ProfileRow {
    locale: String,
    theme_id: Option<String>,
    stack: Option<String>,
    created_at: Option<DateTime<Utc>>,
}

pub async fn profile(
    executor: impl PgExecutor<'_>,
    user_id: Uuid,
) -> Result<LearnerProfile, sqlx::Error> {
    let row = sqlx::query_as::<_, ProfileRow>(
        "SELECT u.locale, p.theme_id, p.stack, p.created_at
         FROM users u LEFT JOIN learner_projects p ON p.user_id = u.id
         WHERE u.id = $1",
    )
    .bind(user_id)
    .fetch_optional(executor)
    .await?;
    let Some(row) = row else {
        return Ok(LearnerProfile::default());
    };
    let project = match (
        row.theme_id,
        row.stack.as_deref().and_then(Stack::parse),
        row.created_at,
    ) {
        (Some(theme_id), Some(stack), Some(created_at)) => Some(Project {
            theme_id,
            stack,
            created_at,
        }),
        _ => None,
    };
    Ok(LearnerProfile {
        locale: row.locale.parse().unwrap_or_default(),
        project,
    })
}

/// `false` when the learner already has a project; it is never replaced.
pub async fn insert_project(
    executor: impl PgExecutor<'_>,
    user_id: Uuid,
    theme_id: &str,
    stack: Stack,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO learner_projects (user_id, theme_id, stack) VALUES ($1, $2, $3)
         ON CONFLICT (user_id) DO NOTHING",
    )
    .bind(user_id)
    .bind(theme_id)
    .bind(stack.as_str())
    .execute(executor)
    .await?;
    Ok(result.rows_affected() == 1)
}
