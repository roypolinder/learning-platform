use sqlx::PgExecutor;
use uuid::Uuid;

use lp_course_engine::Locale;

use super::domain::{User, UserCredentials};

pub async fn insert(
    executor: impl PgExecutor<'_>,
    email: &str,
    display_name: &str,
    password_hash: &str,
    locale: Locale,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "INSERT INTO users (id, email, display_name, password_hash, locale)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT DO NOTHING
         RETURNING id, email, display_name, role, locale, created_at",
    )
    .bind(Uuid::now_v7())
    .bind(email)
    .bind(display_name)
    .bind(password_hash)
    .bind(locale.as_str())
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
        "SELECT id, email, display_name, role, locale, created_at FROM users WHERE id = $1",
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

/// Updates the given profile fields; `None` leaves a field unchanged.
pub async fn update_profile(
    executor: impl PgExecutor<'_>,
    id: Uuid,
    display_name: Option<&str>,
    locale: Option<Locale>,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "UPDATE users
         SET display_name = COALESCE($2, display_name), locale = COALESCE($3, locale),
             updated_at = now()
         WHERE id = $1
         RETURNING id, email, display_name, role, locale, created_at",
    )
    .bind(id)
    .bind(display_name)
    .bind(locale.map(Locale::as_str))
    .fetch_optional(executor)
    .await
}
