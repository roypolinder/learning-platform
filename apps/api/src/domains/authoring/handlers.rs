use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::Deserialize;
use serde_json::Value;

use super::service::{
    self, AdminCourse, AdminCourseDetail, PublishResult, PublishStatus, ValidationReport,
};
use crate::error::ApiResult;
use crate::http::auth::SuperAdmin;
use crate::http::json::ApiJson;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRequest {
    /// Course definition as YAML (or JSON).
    pub source: String,
}

pub async fn list(
    State(state): State<AppState>,
    _: SuperAdmin,
) -> ApiResult<Json<Vec<AdminCourse>>> {
    Ok(Json(service::list(&state).await?))
}

pub async fn detail(
    State(state): State<AppState>,
    _: SuperAdmin,
    Path(course_id): Path<String>,
) -> ApiResult<Json<AdminCourseDetail>> {
    Ok(Json(service::get(&state, &course_id).await?))
}

pub async fn validate(
    State(state): State<AppState>,
    _: SuperAdmin,
    ApiJson(request): ApiJson<SourceRequest>,
) -> Json<ValidationReport> {
    Json(service::validate(&state, &request.source))
}

pub async fn publish(
    State(state): State<AppState>,
    SuperAdmin(user): SuperAdmin,
    ApiJson(request): ApiJson<SourceRequest>,
) -> ApiResult<(StatusCode, Json<PublishResult>)> {
    let result = service::publish(&state, &request.source, user.id).await?;
    let status = match result.status {
        PublishStatus::Created => StatusCode::CREATED,
        _ => StatusCode::OK,
    };
    Ok((status, Json(result)))
}

pub async fn guide(State(state): State<AppState>, _: SuperAdmin) -> Json<Value> {
    Json(service::guide(&state))
}
