use axum::extract::{FromRef, FromRequestParts, OptionalFromRequestParts};
use axum::http::header::{AUTHORIZATION, COOKIE};
use axum::http::request::Parts;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domains::auth::repository as auth_repository;
use crate::domains::users::domain::ROLE_SUPER_ADMIN;
use crate::error::ApiError;
use crate::security::{SESSION_TOKEN_PREFIX, hash_token};

pub const SESSION_COOKIE: &str = "lp_session";

/// An authenticated browser or API user.
#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub auth_session_id: Uuid,
}

impl CurrentUser {
    pub fn is_super_admin(&self) -> bool {
        self.role == ROLE_SUPER_ADMIN
    }
}

/// A signed-in user with the super admin role.
#[derive(Debug, Clone)]
pub struct SuperAdmin(pub CurrentUser);

impl<S> FromRequestParts<S> for SuperAdmin
where
    PgPool: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = <CurrentUser as FromRequestParts<S>>::from_request_parts(parts, state).await?;
        if user.is_super_admin() {
            Ok(Self(user))
        } else {
            Err(ApiError::Forbidden)
        }
    }
}

impl<S> FromRequestParts<S> for CurrentUser
where
    PgPool: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let token = session_token(parts).ok_or(ApiError::Unauthorized)?;
        let pool = PgPool::from_ref(state);
        auth_repository::find_user_by_token_hash(&pool, &hash_token(&token))
            .await?
            .ok_or(ApiError::Unauthorized)
    }
}

impl<S> OptionalFromRequestParts<S> for CurrentUser
where
    PgPool: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        match <CurrentUser as FromRequestParts<S>>::from_request_parts(parts, state).await {
            Ok(user) => Ok(Some(user)),
            Err(ApiError::Unauthorized) => Ok(None),
            Err(error) => Err(error),
        }
    }
}

pub fn session_token(parts: &Parts) -> Option<String> {
    let bearer = parts
        .headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|token| token.starts_with(SESSION_TOKEN_PREFIX));
    if let Some(token) = bearer {
        return Some(token.to_owned());
    }
    parts
        .headers
        .get_all(COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|header| header.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, _)| *name == SESSION_COOKIE)
        .map(|(_, value)| value.to_owned())
}

pub fn session_cookie(token: &str, max_age_seconds: i64, secure: bool) -> String {
    let secure = if secure { "; Secure" } else { "" };
    format!(
        "{SESSION_COOKIE}={token}; HttpOnly; SameSite=Lax; Path=/; Max-Age={max_age_seconds}{secure}"
    )
}

pub fn clear_session_cookie(secure: bool) -> String {
    session_cookie("", 0, secure)
}
