use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct IntegrationConnection {
    pub id: Uuid,
    pub kind: String,
    /// `token` for manually created tokens, `oauth` for grants from the sign-in flow.
    pub auth_method: String,
    pub label: String,
    pub token_hint: String,
    pub status: String,
    pub client_name: Option<String>,
    pub last_session_id: Option<Uuid>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

/// The identity behind an authenticated MCP request.
#[derive(Debug, Clone)]
pub struct McpPrincipal {
    pub user_id: Uuid,
    pub connection_id: Uuid,
    pub is_super_admin: bool,
}
