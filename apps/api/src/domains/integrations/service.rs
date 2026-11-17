use chrono::{Duration, Utc};
use serde::Serialize;
use serde_json::{Value, json};
use uuid::Uuid;

use super::domain::IntegrationConnection;
use super::repository;
use crate::error::{ApiError, ApiResult, FieldError};
use crate::security::{MCP_TOKEN_PREFIX, generate_token, hash_token, token_hint};
use crate::state::AppState;

const TOKEN_LIFETIME: Duration = Duration::days(90);
pub const MAX_ACTIVE_CONNECTIONS: usize = 20;

#[derive(Debug, Serialize)]
pub struct McpConnectionCreated {
    pub connection: IntegrationConnection,
    /// Shown exactly once; only its hash is stored.
    pub token: String,
    pub endpoint: String,
}

pub fn mcp_endpoint(state: &AppState) -> String {
    format!("{}/mcp", state.config.public_base_url)
}

pub async fn create_mcp_connection(
    state: &AppState,
    user_id: Uuid,
    label: &str,
) -> ApiResult<McpConnectionCreated> {
    let label = label.trim();
    if label.is_empty() || label.chars().count() > 60 {
        return Err(ApiError::Validation(vec![FieldError::new(
            "label",
            "must be between 1 and 60 characters",
        )]));
    }
    let existing = repository::list_for_user(&state.db, user_id).await?;
    if existing.iter().filter(|c| c.revoked_at.is_none()).count() >= MAX_ACTIVE_CONNECTIONS {
        return Err(ApiError::Conflict {
            code: "too_many_connections",
            message: "Revoke an unused connection before creating another.".into(),
        });
    }
    let token = generate_token(MCP_TOKEN_PREFIX).map_err(ApiError::internal)?;
    let connection = repository::insert(
        &state.db,
        user_id,
        label,
        &hash_token(&token),
        &token_hint(&token),
        Utc::now() + TOKEN_LIFETIME,
    )
    .await?;
    let endpoint = mcp_endpoint(state);
    Ok(McpConnectionCreated {
        connection,
        token,
        endpoint,
    })
}

/// Project-root `.mcp.json` entry. It carries no credentials: the client discovers the
/// OAuth endpoints from the server's 401 challenge.
pub fn mcp_config(endpoint: &str) -> Value {
    json!({
        "mcpServers": {
            "learning-platform": {
                "type": "http",
                "url": endpoint,
                "tools": ["*"]
            }
        }
    })
}
