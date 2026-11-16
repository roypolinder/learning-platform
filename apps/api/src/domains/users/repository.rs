use sqlx::PgExecutor;
use uuid::Uuid;

use super::domain::{User, UserCredentials};

pub async fn insert(
    executor: impl PgExecutor<'_>,
    email: &str,
    display_name: &str,
    password_hash: &str,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "INSERT INTO users (id, email, display_name, password_hash)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT DO NOTHING
         RETURNING id, email, display_name, role, created_at",
    )
    .bind(Uuid::now_v7())
    .bind(email)
    .bind(display_name)
    .bind(password_hash)
    .fetch_optional(executor)
    .await
}

pub async fn find_credentials_by_email(
    executor: impl PgExecutor<'_>,
    email: &str,
) -> Result<Option<UserCredentials>, sqlx::Error> {
    sqlx::query_as::<_, UserCredentials>(
        "SELECT id, password_hash FROM users WHERE lower(email) = lower($1)",
    )
    .bind(email)
    .fetch_optional(executor)
    .await
}

pub async fn find_by_id(
    executor: impl PgExecutor<'_>,
    id: Uuid,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "SELECT id, email, display_name, role, created_at FROM users WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(executor)
    .await
}

/// Promotes the given (lowercased) emails to super admin. Never demotes.
pub async fn promote_super_admins(
    executor: impl PgExecutor<'_>,
    emails: &[String],
) -> Result<u64, sqlx::Error> {
    if emails.is_empty() {
        return Ok(0);
    }
    let result = sqlx::query(
        "UPDATE users SET role = 'super_admin', updated_at = now()
         WHERE lower(email) = ANY($1) AND role <> 'super_admin'",
    )
    .bind(emails)
    .execute(executor)
    .await?;
    Ok(result.rows_affected())
}

pub async fn update_display_name(
    executor: impl PgExecutor<'_>,
    id: Uuid,
    display_name: &str,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "UPDATE users SET display_name = $2, updated_at = now() WHERE id = $1
         RETURNING id, email, display_name, role, created_at",
    )
    .bind(id)
    .bind(display_name)
    .fetch_optional(executor)
    .await
}
