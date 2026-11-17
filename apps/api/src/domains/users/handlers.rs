use axum::Json;
use axum::extract::State;

use super::dto::{UpdateProfileRequest, UserDto};
use super::repository;
use crate::error::{ApiError, ApiResult, FieldError};
use crate::http::auth::CurrentUser;
use crate::http::json::ApiJson;
use crate::state::AppState;

pub async fn me(State(state): State<AppState>, user: CurrentUser) -> ApiResult<Json<UserDto>> {
    let user = repository::find_by_id(&state.db, user.id)
        .await?
        .ok_or(ApiError::Unauthorized)?;
    Ok(Json(user.into()))
}

#[derive(serde::Serialize)]
pub struct SessionResponse {
    user: Option<UserDto>,
}

/// Like `me`, but signed-out visitors get `200 {"user": null}` instead of a 401 that browsers
/// log as an error on every page load.
pub async fn session(
    State(state): State<AppState>,
    user: Option<CurrentUser>,
) -> ApiResult<Json<SessionResponse>> {
    let user = match user {
        Some(user) => repository::find_by_id(&state.db, user.id)
            .await?
            .map(Into::into),
        None => None,
    };
    Ok(Json(SessionResponse { user }))
}

pub async fn update_me(
    State(state): State<AppState>,
    user: CurrentUser,
    ApiJson(request): ApiJson<UpdateProfileRequest>,
) -> ApiResult<Json<UserDto>> {
    let display_name = request.display_name.as_deref().map(str::trim);
    if display_name.is_some_and(|name| name.is_empty() || name.chars().count() > 80) {
        return Err(ApiError::Validation(vec![FieldError::new(
            "display_name",
            "must be between 1 and 80 characters",
        )]));
    }
    let user = repository::update_profile(&state.db, user.id, display_name, request.locale)
        .await?
        .ok_or(ApiError::Unauthorized)?;
    Ok(Json(user.into()))
}
