use axum::Json;
use axum::extract::{ConnectInfo, State};
use axum::http::header::SET_COOKIE;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{AppendHeaders, IntoResponse};
use std::net::SocketAddr;

use super::dto::{AuthResponse, LoginRequest, RegisterRequest};
use super::repository;
use super::service::{self, IssuedSession, SESSION_LIFETIME};
use crate::error::ApiResult;
use crate::http::auth::{CurrentUser, clear_session_cookie, session_cookie};
use crate::http::client_ip::client_ip;
use crate::http::json::ApiJson;
use crate::state::AppState;

pub async fn register(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    ApiJson(request): ApiJson<RegisterRequest>,
) -> ApiResult<impl IntoResponse> {
    let ip = client_ip(&state.config, peer, &headers);
    state.auth_limiter.check(&format!("register:{ip}"))?;
    let issued = service::register(&state, request).await?;
    Ok((StatusCode::CREATED, session_response(&state, issued)))
}

pub async fn login(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    ApiJson(request): ApiJson<LoginRequest>,
) -> ApiResult<impl IntoResponse> {
    let ip = client_ip(&state.config, peer, &headers);
    state.auth_limiter.check(&format!("login:{ip}"))?;
    state.auth_limiter.check(&format!(
        "login-email:{}",
        request.email.trim().to_lowercase()
    ))?;
    let issued = service::login(&state, request).await?;
    Ok(session_response(&state, issued))
}

pub async fn logout(
    State(state): State<AppState>,
    user: Option<CurrentUser>,
) -> ApiResult<impl IntoResponse> {
    if let Some(user) = user {
        repository::revoke_session(&state.db, user.auth_session_id).await?;
    }
    let mut headers = HeaderMap::new();
    if let Ok(value) = clear_session_cookie(state.config.cookie_secure).parse() {
        headers.insert(SET_COOKIE, value);
    }
    Ok((StatusCode::NO_CONTENT, headers))
}

fn session_response(state: &AppState, issued: IssuedSession) -> impl IntoResponse + use<> {
    let cookie = session_cookie(
        &issued.token,
        SESSION_LIFETIME.num_seconds(),
        state.config.cookie_secure,
    );
    (
        AppendHeaders([(SET_COOKIE, cookie)]),
        Json(AuthResponse {
            user: issued.user.into(),
        }),
    )
}
