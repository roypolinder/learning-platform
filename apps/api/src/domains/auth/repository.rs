use chrono::{DateTime, Utc};
use sqlx::PgExecutor;
use uuid::Uuid;

use crate::http::auth::CurrentUser;

pub async fn insert_session(
    executor: impl PgExecutor<'_>,
    user_id: Uuid,
    token_hash: &[u8],
    expires_at: DateTime<Utc>,
) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO auth_sessions (id, user_id, token_hash, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(id)
    .bind(user_id)
    .bind(token_hash)
    .bind(expires_at)
    .execute(executor)
    .await?;
    Ok(id)
}

pub async fn find_user_by_token_hash(
    executor: impl PgExecutor<'_>,
    token_hash: &[u8],
) -> Result<Option<CurrentUser>, sqlx::Error> {
    let row: Option<(Uuid, String, String, String, String, Uuid)> = sqlx::query_as(
        "SELECT u.id, u.email, u.display_name, u.role, u.locale, s.id
         FROM auth_sessions s JOIN users u ON u.id = s.user_id
         WHERE s.token_hash = $1 AND s.revoked_at IS NULL AND s.expires_at > now()",
    )
    .bind(token_hash)
    .fetch_optional(executor)
    .await?;
    Ok(row.map(
        |(id, email, display_name, role, locale, auth_session_id)| CurrentUser {
            id,
            email,
            display_name,
            role,
            locale,
            auth_session_id,
        },
    ))
}

pub async fn revoke_session(executor: impl PgExecutor<'_>, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE auth_sessions SET revoked_at = now() WHERE id = $1 AND revoked_at IS NULL")
        .bind(id)
        .execute(executor)
        .await?;
    Ok(())
}
