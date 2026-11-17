use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use lp_course_engine::ThemeSummary;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::domain::{Project, Stack, request_locale};
use super::repository;
use crate::error::{ApiError, ApiResult, FieldError};
use crate::http::auth::CurrentUser;
use crate::http::json::ApiJson;
use crate::state::AppState;

#[derive(Serialize)]
pub struct ThemesResponse {
    items: Vec<ThemeSummary>,
}

pub async fn themes(
    State(state): State<AppState>,
    user: Option<CurrentUser>,
    headers: HeaderMap,
) -> Json<ThemesResponse> {
    let locale = request_locale(user.as_ref(), &headers);
    Json(ThemesResponse {
        items: state
            .themes
            .selectable()
            .iter()
            .map(|theme| theme.summary(locale))
            .collect(),
    })
}

#[derive(Serialize)]
pub struct ProjectResponse {
    project: Option<ProjectDto>,
}

#[derive(Serialize)]
pub struct ProjectDto {
    #[serde(flatten)]
    project: Project,
    theme: Option<ThemeSummary>,
}

pub async fn project(
    State(state): State<AppState>,
    user: CurrentUser,
) -> ApiResult<Json<ProjectResponse>> {
    let profile = repository::profile(&state.db, user.id).await?;
    Ok(Json(ProjectResponse {
        project: profile.project.map(|project| ProjectDto {
            theme: state
                .themes
                .get(&project.theme_id)
                .map(|theme| theme.summary(profile.locale)),
            project,
        }),
    }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateProjectRequest {
    theme_id: String,
    stack: Stack,
}

pub async fn create_project(
    State(state): State<AppState>,
    user: CurrentUser,
    ApiJson(request): ApiJson<CreateProjectRequest>,
) -> ApiResult<Json<ProjectResponse>> {
    if state.themes.get(&request.theme_id).is_none() {
        return Err(ApiError::Validation(vec![FieldError::new(
            "theme_id",
            "is not one of the available website types",
        )]));
    }
    let created =
        repository::insert_project(&state.db, user.id, &request.theme_id, request.stack).await?;
    if !created {
        return Err(ApiError::Conflict {
            code: "project_exists",
            message: "You already chose your project. It stays the same for the whole track."
                .into(),
        });
    }
    tracing::info!(user_id = %user.id, theme = %request.theme_id, "learner project created");
    project(State(state), user).await
}

/// Seed data the learner loads into their own site, matching the hidden checks.
pub async fn project_data(
    State(state): State<AppState>,
    user: CurrentUser,
) -> ApiResult<Json<Value>> {
    let profile = repository::profile(&state.db, user.id).await?;
    let project = profile.project.ok_or(ApiError::NotFound("project"))?;
    let theme = state
        .themes
        .get(&project.theme_id)
        .ok_or(ApiError::NotFound("project theme"))?;
    Ok(Json(theme.data().clone()))
}
