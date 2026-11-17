use std::collections::BTreeSet;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use lp_course_engine::view::{course_detail, course_summary};

use super::catalog::LoadedCourse;
use super::dto::{CourseDetailResponse, CourseListQuery, CourseListResponse};
use crate::domains::learners::domain::LearnerProfile;
use crate::domains::learners::repository as learners;
use crate::domains::progress::service as progress;
use crate::domains::sessions::{dto::SessionDto, repository as sessions};
use crate::error::{ApiError, ApiResult};
use crate::http::auth::CurrentUser;
use crate::http::pagination::{Page, PageParams};
use crate::state::AppState;

/// Courses are shown in the viewer's language with their project filled in.
async fn viewer_profile(
    state: &AppState,
    user: Option<&CurrentUser>,
    headers: &HeaderMap,
) -> ApiResult<LearnerProfile> {
    match user {
        Some(user) => Ok(learners::profile(&state.db, user.id).await?),
        None => Ok(LearnerProfile::anonymous(headers)),
    }
}

fn render_for(
    state: &AppState,
    loaded: &LoadedCourse,
    profile: &LearnerProfile,
) -> ApiResult<Arc<LoadedCourse>> {
    state
        .catalog
        .render(&state.engine, loaded, profile.audience(&state.themes))
}

pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<CourseListQuery>,
    user: Option<CurrentUser>,
    headers: HeaderMap,
) -> ApiResult<Json<CourseListResponse>> {
    let profile = viewer_profile(&state, user.as_ref(), &headers).await?;
    let courses = state
        .catalog
        .list()
        .iter()
        .map(|loaded| render_for(&state, loaded, &profile))
        .collect::<ApiResult<Vec<_>>>()?;
    let categories: BTreeSet<String> = courses.iter().map(|c| c.course.category.clone()).collect();
    let needle = query
        .q
        .as_deref()
        .map(str::trim)
        .unwrap_or_default()
        .to_lowercase();

    let matching = courses
        .iter()
        .map(|loaded| &loaded.course)
        .filter(|course| {
            query
                .category
                .as_ref()
                .is_none_or(|category| &course.category == category)
        })
        .filter(|course| {
            query.difficulty.as_ref().is_none_or(|difficulty| {
                serde_json::to_value(course.difficulty)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .as_deref()
                    == Some(difficulty.as_str())
            })
        })
        .filter(|course| {
            needle.is_empty()
                || course.title.to_lowercase().contains(&needle)
                || course.summary.to_lowercase().contains(&needle)
                || course
                    .tags
                    .iter()
                    .any(|tag| tag.to_lowercase().contains(&needle))
        })
        .map(course_summary)
        .collect();

    let params = PageParams {
        page: query.page,
        per_page: query.per_page,
    };
    let progress = match user {
        Some(user) => progress::course_progress(&state, user.id).await?,
        None => Default::default(),
    };
    Ok(Json(CourseListResponse {
        page: Page::slice(matching, params),
        categories: categories.into_iter().collect(),
        progress,
    }))
}

pub async fn detail(
    State(state): State<AppState>,
    Path(course_id): Path<String>,
    user: Option<CurrentUser>,
    headers: HeaderMap,
) -> ApiResult<Json<CourseDetailResponse>> {
    let template = state
        .catalog
        .current(&course_id)
        .ok_or(ApiError::NotFound("course"))?;
    let profile = viewer_profile(&state, user.as_ref(), &headers).await?;
    let loaded = render_for(&state, &template, &profile)?;
    let session = match user {
        Some(user) => sessions::find_latest_for_course(&state.db, user.id, loaded.course_uuid)
            .await?
            .map(|session| SessionDto::from(&session)),
        None => None,
    };
    Ok(Json(CourseDetailResponse {
        course: course_detail(&state.engine, &loaded.course),
        session,
    }))
}

/// Fixtures that a learner's browser sends to their own service during service checks.
pub async fn asset(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};

    if !lp_extensions::service::is_asset_name(&name) {
        return Err(ApiError::NotFound("asset"));
    }
    let path = state.config.courses_dir.join("assets").join(&name);
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|_| ApiError::NotFound("asset"))?;
    let mime = match path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "pdf" => "application/pdf",
        "wav" => "audio/wav",
        "mp3" => "audio/mpeg",
        "json" => "application/json",
        "txt" | "md" | "csv" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    };
    Ok((
        [
            (CONTENT_TYPE, mime),
            (CACHE_CONTROL, "public, max-age=3600"),
        ],
        bytes,
    ))
}
