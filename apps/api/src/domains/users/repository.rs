use sqlx::PgExecutor;
use uuid::Uuid;

use lp_course_engine::Locale;

use super::domain::{User, UserCredentials};

/// Columns of [`User`]. `pending_email` is the newest unconfirmed email change.
macro_rules! user_columns {
    () => {
        "id, email, display_name, role, locale, created_at, email_verified_at,
         (SELECT t.email FROM email_tokens t
          WHERE t.user_id = users.id AND t.purpose = 'change_email'
            AND t.used_at IS NULL AND t.expires_at > now()
          ORDER BY t.created_at DESC LIMIT 1) AS pending_email"
    };
}

pub async fn insert(
    executor: impl PgExecutor<'_>,
    email: &str,
    display_name: &str,
    password_hash: &str,
    locale: Locale,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(concat!(
        "INSERT INTO users (id, email, display_name, password_hash, locale)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT DO NOTHING
         RETURNING ",
        user_columns!()
    ))
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
    sqlx::query_as::<_, User>(concat!(
        "SELECT ",
        user_columns!(),
        " FROM users WHERE id = $1"
    ))
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
    sqlx::query_as::<_, User>(concat!(
        "UPDATE users
         SET display_name = COALESCE($2, display_name), locale = COALESCE($3, locale),
             updated_at = now()
         WHERE id = $1
         RETURNING ",
        user_columns!()
    ))
    .bind(id)
    .bind(display_name)
    .bind(locale.map(Locale::as_str))
    .fetch_optional(executor)
    .await
}

pub async fn find_by_email(
    executor: impl PgExecutor<'_>,
    email: &str,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(concat!(
        "SELECT ",
        user_columns!(),
        " FROM users WHERE lower(email) = lower($1)"
    ))
    .bind(email)
    .fetch_optional(executor)
    .await
}

pub async fn find_password_hash(
    executor: impl PgExecutor<'_>,
    id: Uuid,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(executor)
        .await
}

pub async fn email_taken(executor: impl PgExecutor<'_>, email: &str) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM users WHERE lower(email) = lower($1))")
        .bind(email)
        .fetch_one(executor)
        .await
}

/// Marks the address verified, but only while it is still the account's address.
pub async fn mark_email_verified(
    executor: impl PgExecutor<'_>,
    id: Uuid,
    email: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE users SET email_verified_at = COALESCE(email_verified_at, now()), updated_at = now()
         WHERE id = $1 AND lower(email) = lower($2)",
    )
    .bind(id)
    .bind(email)
    .execute(executor)
    .await?;
    Ok(result.rows_affected() == 1)
}

/// Switches to a confirmed new address. `None` when another account already uses it.
pub async fn change_email(
    executor: impl PgExecutor<'_>,
    id: Uuid,
    email: &str,
) -> Result<Option<()>, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE users SET email = $2, email_verified_at = now(), updated_at = now() WHERE id = $1",
    )
    .bind(id)
    .bind(email)
    .execute(executor)
    .await;
    match result {
        Ok(_) => Ok(Some(())),
        Err(sqlx::Error::Database(error)) if error.is_unique_violation() => Ok(None),
        Err(error) => Err(error),
    }
}

pub async fn set_password_hash(
    executor: impl PgExecutor<'_>,
    id: Uuid,
    password_hash: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET password_hash = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(password_hash)
        .execute(executor)
        .await?;
    Ok(())
}

/// Counts super admins and locks their rows for the rest of the transaction.
pub async fn count_super_admins(executor: impl PgExecutor<'_>) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT count(*) FROM (SELECT 1 FROM users WHERE role = 'super_admin' FOR UPDATE) admins",
    )
    .fetch_one(executor)
    .await
}

/// Deletes one account. Everything the user owns is removed by `ON DELETE CASCADE`.
pub async fn delete(executor: impl PgExecutor<'_>, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() == 1)
}
