use std::net::SocketAddr;

use axum::Json;
use axum::extract::{ConnectInfo, State};
use axum::http::header::SET_COOKIE;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;

use super::dto::{
    ChangeEmailRequest, ChangePasswordRequest, ConfirmEmailRequest, DeleteAccountRequest,
    ForgotPasswordRequest, ResetPasswordRequest, TokenResult,
};
use super::service;
use crate::domains::users::dto::UserDto;
use crate::error::ApiResult;
use crate::http::auth::{CurrentUser, clear_session_cookie};
use crate::http::client_ip::client_ip;
use crate::http::json::ApiJson;
use crate::state::AppState;

pub async fn forgot_password(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    ApiJson(request): ApiJson<ForgotPasswordRequest>,
) -> ApiResult<StatusCode> {
    let ip = client_ip(&state.config, peer, &headers);
    state.auth_limiter.check(&format!("forgot:{ip}"))?;
    service::forgot_password(&state, &request.email);
    Ok(StatusCode::ACCEPTED)
}

pub async fn reset_password(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    ApiJson(request): ApiJson<ResetPasswordRequest>,
) -> ApiResult<StatusCode> {
    let ip = client_ip(&state.config, peer, &headers);
    state.auth_limiter.check(&format!("email-link:{ip}"))?;
    service::reset_password(&state, request).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Works without a session: the link itself is the proof, and it may be opened on another
/// device than the one the learner signed in on.
pub async fn confirm_email(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    ApiJson(request): ApiJson<ConfirmEmailRequest>,
) -> ApiResult<Json<TokenResult>> {
    let ip = client_ip(&state.config, peer, &headers);
    state.auth_limiter.check(&format!("email-link:{ip}"))?;
    Ok(Json(service::confirm_email(&state, &request.token).await?))
}

pub async fn resend_verification(
    State(state): State<AppState>,
    user: CurrentUser,
) -> ApiResult<StatusCode> {
    service::resend_verification(&state, user.id).await?;
    Ok(StatusCode::ACCEPTED)
}

pub async fn change_email(
    State(state): State<AppState>,
    user: CurrentUser,
    ApiJson(request): ApiJson<ChangeEmailRequest>,
) -> ApiResult<(StatusCode, Json<UserDto>)> {
    let user = service::request_email_change(&state, user.id, request).await?;
    Ok((StatusCode::ACCEPTED, Json(user.into())))
}

pub async fn cancel_email_change(
    State(state): State<AppState>,
    user: CurrentUser,
) -> ApiResult<Json<UserDto>> {
    Ok(Json(
        service::cancel_email_change(&state, user.id).await?.into(),
    ))
}

pub async fn change_password(
    State(state): State<AppState>,
    user: CurrentUser,
    ApiJson(request): ApiJson<ChangePasswordRequest>,
) -> ApiResult<StatusCode> {
    service::change_password(&state, user.id, user.auth_session_id, request).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Deletes the account of the signed-in user. There is deliberately no variant that takes a
/// user id, so nobody can delete someone else's account through this API.
pub async fn delete_account(
    State(state): State<AppState>,
    user: CurrentUser,
    ApiJson(request): ApiJson<DeleteAccountRequest>,
) -> ApiResult<impl IntoResponse> {
    service::delete_account(&state, user.id, request).await?;
    let mut headers = HeaderMap::new();
    if let Ok(value) = clear_session_cookie(state.config.cookie_secure).parse() {
        headers.insert(SET_COOKIE, value);
    }
    Ok((StatusCode::NO_CONTENT, headers))
}
