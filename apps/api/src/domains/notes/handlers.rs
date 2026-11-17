use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::repository::{self, Note};
use crate::error::{ApiError, ApiResult, FieldError};
use crate::http::auth::CurrentUser;
use crate::http::json::ApiJson;
use crate::state::AppState;

const MAX_LENGTH: usize = 5000;

#[derive(Deserialize)]
pub struct ListQuery {
    /// Omit to list notes of every course, e.g. to look something up from another one.
    course: Option<String>,
}

#[derive(Serialize)]
pub struct NotesResponse {
    items: Vec<Note>,
}

pub async fn list(
    State(state): State<AppState>,
    user: CurrentUser,
    Query(query): Query<ListQuery>,
) -> ApiResult<Json<NotesResponse>> {
    let items = repository::list(&state.db, user.id, query.course.as_deref()).await?;
    Ok(Json(NotesResponse { items }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateNote {
    course: String,
    body: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateNote {
    body: String,
}

pub async fn create(
    State(state): State<AppState>,
    user: CurrentUser,
    ApiJson(request): ApiJson<CreateNote>,
) -> ApiResult<(StatusCode, Json<Note>)> {
    if state.catalog.current(&request.course).is_none() {
        return Err(ApiError::NotFound("course"));
    }
    let body = valid_body(&request.body)?;
    let note = repository::insert(&state.db, user.id, &request.course, body).await?;
    Ok((StatusCode::CREATED, Json(note)))
}

pub async fn update(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    ApiJson(request): ApiJson<UpdateNote>,
) -> ApiResult<Json<Note>> {
    let body = valid_body(&request.body)?;
    repository::update(&state.db, user.id, id, body)
        .await?
        .map(Json)
        .ok_or(ApiError::NotFound("note"))
}

pub async fn remove(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    if repository::delete(&state.db, user.id, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound("note"))
    }
}

fn valid_body(body: &str) -> ApiResult<&str> {
    let body = body.trim();
    if body.is_empty() || body.chars().count() > MAX_LENGTH {
        return Err(ApiError::Validation(vec![FieldError::new(
            "body",
            format!("must be between 1 and {MAX_LENGTH} characters"),
        )]));
    }
    Ok(body)
}
