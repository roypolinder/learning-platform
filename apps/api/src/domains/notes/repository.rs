use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgExecutor;
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Note {
    pub id: Uuid,
    pub course_slug: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Notes for one course, or all of the user's notes when `course` is `None`.
pub async fn list(
    executor: impl PgExecutor<'_>,
    user_id: Uuid,
    course: Option<&str>,
) -> Result<Vec<Note>, sqlx::Error> {
    sqlx::query_as::<_, Note>(
        "SELECT id, course_slug, body, created_at, updated_at FROM course_notes
         WHERE user_id = $1 AND ($2::text IS NULL OR course_slug = $2)
         ORDER BY course_slug, created_at
         LIMIT 1000",
    )
    .bind(user_id)
    .bind(course)
    .fetch_all(executor)
    .await
}

pub async fn insert(
    executor: impl PgExecutor<'_>,
    user_id: Uuid,
    course: &str,
    body: &str,
) -> Result<Note, sqlx::Error> {
    sqlx::query_as::<_, Note>(
        "INSERT INTO course_notes (id, user_id, course_slug, body) VALUES ($1, $2, $3, $4)
         RETURNING id, course_slug, body, created_at, updated_at",
    )
    .bind(Uuid::now_v7())
    .bind(user_id)
    .bind(course)
    .bind(body)
    .fetch_one(executor)
    .await
}

pub async fn update(
    executor: impl PgExecutor<'_>,
    user_id: Uuid,
    id: Uuid,
    body: &str,
) -> Result<Option<Note>, sqlx::Error> {
    sqlx::query_as::<_, Note>(
        "UPDATE course_notes SET body = $3, updated_at = now() WHERE id = $1 AND user_id = $2
         RETURNING id, course_slug, body, created_at, updated_at",
    )
    .bind(id)
    .bind(user_id)
    .bind(body)
    .fetch_optional(executor)
    .await
}

pub async fn delete(
    executor: impl PgExecutor<'_>,
    user_id: Uuid,
    id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM course_notes WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() == 1)
}
