use chrono::{DateTime, Utc};
use sqlx::PgExecutor;
use uuid::Uuid;

use super::domain::{ConsumedToken, TokenPurpose};

pub async fn insert_token(
    executor: impl PgExecutor<'_>,
    user_id: Uuid,
    purpose: TokenPurpose,
    email: &str,
    token_hash: &[u8],
    expires_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO email_tokens (id, user_id, purpose, email, token_hash, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(Uuid::now_v7())
    .bind(user_id)
    .bind(purpose.as_str())
    .bind(email)
    .bind(token_hash)
    .bind(expires_at)
    .execute(executor)
    .await?;
    Ok(())
}

/// Marks a valid link as used and returns it. Atomic, so a link works exactly once.
pub async fn consume_token(
    executor: impl PgExecutor<'_>,
    token_hash: &[u8],
    purposes: &[TokenPurpose],
) -> Result<Option<ConsumedToken>, sqlx::Error> {
    let purposes: Vec<&str> = purposes.iter().map(|purpose| purpose.as_str()).collect();
    sqlx::query_as::<_, ConsumedToken>(
        "UPDATE email_tokens SET used_at = now()
         WHERE token_hash = $1 AND purpose = ANY($2) AND used_at IS NULL AND expires_at > now()
         RETURNING user_id, purpose, email",
    )
    .bind(token_hash)
    .bind(purposes)
    .fetch_optional(executor)
    .await
}

/// Retires every open link of one kind, e.g. older reset links after a successful reset.
pub async fn invalidate_tokens(
    executor: impl PgExecutor<'_>,
    user_id: Uuid,
    purpose: TokenPurpose,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE email_tokens SET used_at = now()
         WHERE user_id = $1 AND purpose = $2 AND used_at IS NULL",
    )
    .bind(user_id)
    .bind(purpose.as_str())
    .execute(executor)
    .await?;
    Ok(result.rows_affected())
}

/// Signs the account out everywhere, optionally except the current browser session.
pub async fn revoke_sessions(
    executor: impl PgExecutor<'_>,
    user_id: Uuid,
    except: Option<Uuid>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE auth_sessions SET revoked_at = now()
         WHERE user_id = $1 AND revoked_at IS NULL AND id IS DISTINCT FROM $2",
    )
    .bind(user_id)
    .bind(except)
    .execute(executor)
    .await?;
    Ok(())
}
