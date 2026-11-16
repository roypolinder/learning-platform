use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::domain::IntegrationConnection;
use super::repository;
use super::service::{self, McpConnectionCreated};
use crate::error::{ApiError, ApiResult};
use crate::http::auth::CurrentUser;
use crate::http::json::ApiJson;
use crate::state::AppState;

#[derive(Serialize)]
pub struct IntegrationsResponse {
    pub mcp_endpoint: String,
    pub mcp_config: Value,
    pub connections: Vec<IntegrationConnection>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateMcpConnectionRequest {
    pub label: String,
}

pub async fn list(
    State(state): State<AppState>,
    user: CurrentUser,
) -> ApiResult<Json<IntegrationsResponse>> {
    let endpoint = service::mcp_endpoint(&state);
    Ok(Json(IntegrationsResponse {
        mcp_config: service::mcp_config(&endpoint),
        mcp_endpoint: endpoint,
        connections: repository::list_for_user(&state.db, user.id).await?,
    }))
}

pub async fn create_mcp(
    State(state): State<AppState>,
    user: CurrentUser,
    ApiJson(request): ApiJson<CreateMcpConnectionRequest>,
) -> ApiResult<(StatusCode, Json<McpConnectionCreated>)> {
    let created = service::create_mcp_connection(&state, user.id, &request.label).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn revoke(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(connection_id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    if repository::revoke(&state.db, connection_id, user.id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound("connection"))
    }
}
