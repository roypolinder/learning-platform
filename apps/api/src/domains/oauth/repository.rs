use chrono::{DateTime, Utc};
use sqlx::PgExecutor;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OAuthClient {
    pub id: Uuid,
    pub client_name: String,
    pub redirect_uris: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct AuthorizationCode {
    pub id: Uuid,
    pub client_id: Uuid,
    pub user_id: Uuid,
    pub redirect_uri: String,
    pub code_challenge: String,
}

pub async fn insert_client(
    executor: impl PgExecutor<'_>,
    client_name: &str,
    redirect_uris: &[String],
) -> Result<OAuthClient, sqlx::Error> {
    sqlx::query_as(
        "INSERT INTO oauth_clients (id, client_name, redirect_uris) VALUES ($1, $2, $3)
         RETURNING id, client_name, redirect_uris, created_at",
    )
    .bind(Uuid::now_v7())
    .bind(client_name)
    .bind(redirect_uris)
    .fetch_one(executor)
    .await
}

pub async fn find_client(
    executor: impl PgExecutor<'_>,
    id: Uuid,
) -> Result<Option<OAuthClient>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, client_name, redirect_uris, created_at FROM oauth_clients WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(executor)
    .await
}

pub struct NewCode<'a> {
    pub code_hash: &'a [u8],
    pub client_id: Uuid,
    pub user_id: Uuid,
    pub redirect_uri: &'a str,
    pub code_challenge: &'a str,
    pub scope: &'a str,
    pub expires_at: DateTime<Utc>,
}

pub async fn insert_code(
    executor: impl PgExecutor<'_>,
    code: &NewCode<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO oauth_authorization_codes
           (id, code_hash, client_id, user_id, redirect_uri, code_challenge, scope, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(Uuid::now_v7())
    .bind(code.code_hash)
    .bind(code.client_id)
    .bind(code.user_id)
    .bind(code.redirect_uri)
    .bind(code.code_challenge)
    .bind(code.scope)
    .bind(code.expires_at)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn delete_stale_codes(executor: impl PgExecutor<'_>) -> Result<(), sqlx::Error> {
    sqlx::query(
        "DELETE FROM oauth_authorization_codes WHERE expires_at < now() - interval '1 day'",
    )
    .execute(executor)
    .await?;
    Ok(())
}

/// Marks a code as used and returns it, atomically, so a code can be exchanged once.
pub async fn consume_code(
    executor: impl PgExecutor<'_>,
    code_hash: &[u8],
) -> Result<Option<AuthorizationCode>, sqlx::Error> {
    sqlx::query_as(
        "UPDATE oauth_authorization_codes SET used_at = now()
         WHERE code_hash = $1 AND used_at IS NULL AND expires_at > now()
         RETURNING id, client_id, user_id, redirect_uri, code_challenge",
    )
    .bind(code_hash)
    .fetch_optional(executor)
    .await
}

/// The connection issued for an already-used code, if any.
pub async fn replayed_code_connection(
    executor: impl PgExecutor<'_>,
    code_hash: &[u8],
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT connection_id FROM oauth_authorization_codes
         WHERE code_hash = $1 AND used_at IS NOT NULL AND connection_id IS NOT NULL",
    )
    .bind(code_hash)
    .fetch_optional(executor)
    .await
}

pub async fn link_code_connection(
    executor: impl PgExecutor<'_>,
    code_id: Uuid,
    connection_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE oauth_authorization_codes SET connection_id = $2 WHERE id = $1")
        .bind(code_id)
        .bind(connection_id)
        .execute(executor)
        .await?;
    Ok(())
}
