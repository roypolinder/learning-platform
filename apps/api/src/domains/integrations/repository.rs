use chrono::{DateTime, Utc};
use sqlx::PgExecutor;
use uuid::Uuid;

use super::domain::{IntegrationConnection, McpPrincipal};

macro_rules! columns {
    () => {
        "id, kind, auth_method, label, token_hint, status, client_name, last_session_id, last_seen_at, created_at, expires_at, revoked_at"
    };
}

pub async fn insert(
    executor: impl PgExecutor<'_>,
    user_id: Uuid,
    label: &str,
    token_hash: &[u8],
    token_hint: &str,
    expires_at: DateTime<Utc>,
) -> Result<IntegrationConnection, sqlx::Error> {
    sqlx::query_as(concat!(
        "INSERT INTO integration_connections (id, user_id, kind, label, token_hash, token_hint, status, expires_at) ",
        "VALUES ($1, $2, 'mcp', $3, $4, $5, 'pending', $6) RETURNING ",
        columns!()
    ))
    .bind(Uuid::now_v7())
    .bind(user_id)
    .bind(label)
    .bind(token_hash)
    .bind(token_hint)
    .bind(expires_at)
    .fetch_one(executor)
    .await
}

/// Hashes and expiry of a freshly issued OAuth access/refresh token pair.
pub struct OAuthTokens<'a> {
    pub access_hash: &'a [u8],
    pub access_hint: &'a str,
    pub refresh_hash: &'a [u8],
    pub access_expires_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

pub async fn insert_oauth(
    executor: impl PgExecutor<'_>,
    user_id: Uuid,
    oauth_client_id: Uuid,
    label: &str,
    tokens: &OAuthTokens<'_>,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO integration_connections
           (id, user_id, kind, auth_method, oauth_client_id, label, token_hash, token_hint,
            refresh_token_hash, status, access_expires_at, expires_at)
         VALUES ($1, $2, 'mcp', 'oauth', $3, $4, $5, $6, $7, 'pending', $8, $9)
         RETURNING id",
    )
    .bind(Uuid::now_v7())
    .bind(user_id)
    .bind(oauth_client_id)
    .bind(label)
    .bind(tokens.access_hash)
    .bind(tokens.access_hint)
    .bind(tokens.refresh_hash)
    .bind(tokens.access_expires_at)
    .bind(tokens.expires_at)
    .fetch_one(executor)
    .await
}

/// Rotates both tokens of an active OAuth grant. The old refresh token stops working.
pub async fn rotate_oauth(
    executor: impl PgExecutor<'_>,
    oauth_client_id: Uuid,
    old_refresh_hash: &[u8],
    tokens: &OAuthTokens<'_>,
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar(
        "UPDATE integration_connections
         SET token_hash = $3, token_hint = $4, refresh_token_hash = $5,
             access_expires_at = $6, expires_at = $7
         WHERE oauth_client_id = $1 AND refresh_token_hash = $2
           AND revoked_at IS NULL AND expires_at > now()
         RETURNING id",
    )
    .bind(oauth_client_id)
    .bind(old_refresh_hash)
    .bind(tokens.access_hash)
    .bind(tokens.access_hint)
    .bind(tokens.refresh_hash)
    .bind(tokens.access_expires_at)
    .bind(tokens.expires_at)
    .fetch_optional(executor)
    .await
}

/// Re-authorizing a client replaces its earlier grant instead of piling up connections.
pub async fn revoke_oauth_client_grants(
    executor: impl PgExecutor<'_>,
    user_id: Uuid,
    oauth_client_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE integration_connections SET status = 'revoked', revoked_at = now()
         WHERE user_id = $1 AND oauth_client_id = $2 AND revoked_at IS NULL",
    )
    .bind(user_id)
    .bind(oauth_client_id)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn revoke_by_id(executor: impl PgExecutor<'_>, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE integration_connections SET status = 'revoked', revoked_at = now()
         WHERE id = $1 AND revoked_at IS NULL",
    )
    .bind(id)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn count_active(
    executor: impl PgExecutor<'_>,
    user_id: Uuid,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT count(*) FROM integration_connections
         WHERE user_id = $1 AND revoked_at IS NULL AND expires_at > now()",
    )
    .bind(user_id)
    .fetch_one(executor)
    .await
}

pub async fn list_for_user(
    executor: impl PgExecutor<'_>,
    user_id: Uuid,
) -> Result<Vec<IntegrationConnection>, sqlx::Error> {
    sqlx::query_as(concat!(
        "SELECT ",
        columns!(),
        " FROM integration_connections WHERE user_id = $1 ORDER BY created_at DESC LIMIT 50"
    ))
    .bind(user_id)
    .fetch_all(executor)
    .await
}

pub async fn revoke(
    executor: impl PgExecutor<'_>,
    id: Uuid,
    user_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE integration_connections SET status = 'revoked', revoked_at = now()
         WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL",
    )
    .bind(id)
    .bind(user_id)
    .execute(executor)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn authenticate(
    executor: impl PgExecutor<'_>,
    token_hash: &[u8],
) -> Result<Option<McpPrincipal>, sqlx::Error> {
    let row: Option<(Uuid, Uuid, bool)> = sqlx::query_as(
        "SELECT c.id, c.user_id, u.role = 'super_admin'
         FROM integration_connections c JOIN users u ON u.id = c.user_id
         WHERE c.token_hash = $1 AND c.kind = 'mcp' AND c.revoked_at IS NULL AND c.expires_at > now()
           AND (c.access_expires_at IS NULL OR c.access_expires_at > now())",
    )
    .bind(token_hash)
    .fetch_optional(executor)
    .await?;
    Ok(
        row.map(|(connection_id, user_id, is_super_admin)| McpPrincipal {
            user_id,
            connection_id,
            is_super_admin,
        }),
    )
}

pub async fn record_activity(
    executor: impl PgExecutor<'_>,
    connection_id: Uuid,
    client_name: Option<&str>,
    session_id: Option<Uuid>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE integration_connections
         SET status = 'connected', last_seen_at = now(),
             client_name = COALESCE($2, client_name),
             last_session_id = COALESCE($3, last_session_id)
         WHERE id = $1 AND revoked_at IS NULL",
    )
    .bind(connection_id)
    .bind(client_name)
    .bind(session_id)
    .execute(executor)
    .await?;
    Ok(())
}
