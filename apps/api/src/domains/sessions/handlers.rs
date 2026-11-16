use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use lp_core::{Submission, SubmissionSource};
use lp_course_engine::view::course_detail;
use uuid::Uuid;

use super::domain::Actor;
use super::dto::{
    ProgressDto, SessionDetailResponse, SessionDto, SessionEventDto, SessionListQuery,
    StepActionResponse, SubmissionRequest,
};
use super::repository;
use super::service::{MutationResult, SessionService, SessionSnapshot};
use crate::error::ApiResult;
use crate::http::auth::CurrentUser;
use crate::http::json::ApiJson;
use crate::http::pagination::{Page, PageParams};
use crate::state::AppState;

pub async fn start(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(course_id): Path<String>,
) -> ApiResult<(StatusCode, Json<SessionDetailResponse>)> {
    let (snapshot, created) = SessionService::new(&state)
        .start(user.id, &course_id)
        .await?;
    let status = if created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((status, Json(detail_response(&state, &snapshot))))
}

pub async fn list(
    State(state): State<AppState>,
    user: CurrentUser,
    Query(query): Query<SessionListQuery>,
) -> ApiResult<Json<Page<SessionDto>>> {
    let params = PageParams {
        page: query.page,
        per_page: query.per_page,
    };
    let sessions = repository::list_for_user(
        &state.db,
        user.id,
        query.status,
        params.limit(),
        params.offset(),
    )
    .await?;
    let total = repository::count_for_user(&state.db, user.id, query.status).await?;
    Ok(Json(Page::new(
        sessions.iter().map(SessionDto::from).collect(),
        params,
        u64::try_from(total).unwrap_or_default(),
    )))
}

pub async fn detail(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(session_id): Path<Uuid>,
) -> ApiResult<Json<SessionDetailResponse>> {
    let service = SessionService::new(&state);
    let mut snapshot = service.load(user.id, session_id).await?;
    // Clients notice a different `session.id` and switch to it.
    if let Some(upgraded) = service
        .upgrade_if_outdated(user.id, &snapshot.session)
        .await?
    {
        snapshot = upgraded;
    }
    service.touch(snapshot.session.id).await?;
    Ok(Json(detail_response(&state, &snapshot)))
}

pub async fn progress(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(session_id): Path<Uuid>,
) -> ApiResult<Json<ProgressDto>> {
    let snapshot = SessionService::new(&state)
        .load(user.id, session_id)
        .await?;
    Ok(Json(progress_dto(&state, &snapshot)))
}

pub async fn events(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(session_id): Path<Uuid>,
    Query(params): Query<PageParams>,
) -> ApiResult<Json<Vec<SessionEventDto>>> {
    repository::find_for_user(&state.db, session_id, user.id)
        .await?
        .ok_or(crate::error::ApiError::NotFound("session"))?;
    let events =
        repository::list_events(&state.db, session_id, params.limit(), params.offset()).await?;
    Ok(Json(
        events.into_iter().map(SessionEventDto::from).collect(),
    ))
}

pub async fn refresh(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(session_id): Path<Uuid>,
) -> ApiResult<Json<ProgressDto>> {
    let snapshot = SessionService::new(&state)
        .refresh(user.id, session_id, Actor::Learner)
        .await?;
    Ok(Json(progress_dto(&state, &snapshot)))
}

pub async fn open_step(
    State(state): State<AppState>,
    user: CurrentUser,
    Path((session_id, step_id)): Path<(Uuid, String)>,
) -> ApiResult<Json<StepActionResponse>> {
    let result = SessionService::new(&state)
        .open_step(user.id, session_id, &step_id)
        .await?;
    Ok(Json(action_response(&state, result)))
}

pub async fn complete_step(
    State(state): State<AppState>,
    user: CurrentUser,
    Path((session_id, step_id)): Path<(Uuid, String)>,
) -> ApiResult<Json<StepActionResponse>> {
    let result = SessionService::new(&state)
        .complete_step(user.id, session_id, &step_id)
        .await?;
    Ok(Json(action_response(&state, result)))
}

pub async fn submit(
    State(state): State<AppState>,
    user: CurrentUser,
    Path((session_id, step_id)): Path<(Uuid, String)>,
    ApiJson(request): ApiJson<SubmissionRequest>,
) -> ApiResult<Json<StepActionResponse>> {
    let submission = Submission {
        source: SubmissionSource::Web,
        payload: request.payload,
    };
    let result = SessionService::new(&state)
        .submit(user.id, session_id, &step_id, submission)
        .await?;
    Ok(Json(action_response(&state, result)))
}

fn progress_dto(state: &AppState, snapshot: &SessionSnapshot) -> ProgressDto {
    ProgressDto::build(
        &state.engine,
        &snapshot.session,
        &snapshot.course.course,
        &snapshot.progress,
    )
}

fn detail_response(state: &AppState, snapshot: &SessionSnapshot) -> SessionDetailResponse {
    SessionDetailResponse {
        session: SessionDto::from(&snapshot.session),
        course: course_detail(&state.engine, &snapshot.course.course),
        progress: progress_dto(state, snapshot),
    }
}

fn action_response(state: &AppState, result: MutationResult) -> StepActionResponse {
    StepActionResponse {
        progress: progress_dto(state, &result.snapshot),
        outcome: result.outcome,
    }
}
