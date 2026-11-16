use axum::Json;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use chrono::{Duration, Utc};
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

use super::protocol::{
    AuthorizationParams, RedirectableError, redirect_uri_matches, redirect_with,
    validate_redirect_uris, validate_request, verify_pkce,
};
use super::repository::{self, NewCode, OAuthClient};
use crate::domains::integrations::repository::{self as connections, OAuthTokens};
use crate::domains::integrations::service::MAX_ACTIVE_CONNECTIONS;
use crate::error::{ApiError, ApiResult};
use crate::security::{MCP_TOKEN_PREFIX, generate_token, hash_token, token_hint};
use crate::state::AppState;

pub const ACCESS_TOKEN_LIFETIME: Duration = Duration::hours(1);
const GRANT_LIFETIME: Duration = Duration::days(90);
const CODE_LIFETIME: Duration = Duration::minutes(5);
const CODE_PREFIX: &str = "lp_code_";
const REFRESH_TOKEN_PREFIX: &str = "lp_rt_";
const MAX_CLIENT_NAME: usize = 100;
const MAX_LABEL: usize = 60;

/// Errors in the RFC 6749 wire format used by the registration and token endpoints.
#[derive(Debug)]
pub struct OAuthError {
    status: StatusCode,
    error: &'static str,
    description: String,
}

impl OAuthError {
    pub fn new(status: StatusCode, error: &'static str, description: impl Into<String>) -> Self {
        Self {
            status,
            error,
            description: description.into(),
        }
    }

    pub fn bad_request(error: &'static str, description: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, error, description)
    }

    fn invalid_grant() -> Self {
        Self::bad_request(
            "invalid_grant",
            "the grant is invalid, expired, revoked or was issued to another client",
        )
    }

    fn server_error(error: impl std::fmt::Display) -> Self {
        tracing::error!(%error, "oauth internal error");
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "server_error",
            "Something went wrong on our side.",
        )
    }
}

impl From<sqlx::Error> for OAuthError {
    fn from(error: sqlx::Error) -> Self {
        Self::server_error(error)
    }
}

impl From<ApiError> for OAuthError {
    fn from(error: ApiError) -> Self {
        match error {
            ApiError::RateLimited { .. } => Self::new(
                StatusCode::TOO_MANY_REQUESTS,
                "slow_down",
                "too many requests",
            ),
            other => Self::server_error(other),
        }
    }
}

impl IntoResponse for OAuthError {
    fn into_response(self) -> Response {
        no_store(
            self.status,
            Json(json!({ "error": self.error, "error_description": self.description })),
        )
    }
}

/// Token endpoint responses must not be cached (RFC 6749 §5.1).
pub fn no_store(status: StatusCode, body: impl IntoResponse) -> Response {
    let mut response = (status, body).into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
        .headers_mut()
        .insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    response
}

/// Why an authorization request cannot proceed.
pub enum AuthorizeError {
    /// The client or redirect URI is untrusted, so the user must not be redirected.
    Fatal(String),
    /// Reported to the client on its verified redirect URI.
    Redirect { location: String },
}

pub struct ValidAuthorization {
    pub client: OAuthClient,
    pub redirect_uri: String,
    pub code_challenge: String,
    pub scope: String,
    pub state: Option<String>,
}

pub async fn validate_authorization(
    state: &AppState,
    params: &AuthorizationParams,
) -> Result<ValidAuthorization, AuthorizeError> {
    let fatal = |message: &str| AuthorizeError::Fatal(message.to_owned());
    let client_id = params
        .client_id
        .as_deref()
        .and_then(|id| Uuid::parse_str(id).ok())
        .ok_or_else(|| fatal("The application did not identify itself correctly."))?;
    let client = repository::find_client(&state.db, client_id)
        .await
        .map_err(|error| AuthorizeError::Fatal(ApiError::from(error).to_string()))?
        .ok_or_else(|| fatal("This application is not registered with the platform."))?;
    let redirect_uri = match params.redirect_uri.as_deref() {
        Some(requested) => client
            .redirect_uris
            .iter()
            .any(|registered| redirect_uri_matches(registered, requested))
            .then(|| requested.to_owned())
            .ok_or_else(|| fatal("The application asked to return to an unregistered address."))?,
        None if client.redirect_uris.len() == 1 => client.redirect_uris[0].clone(),
        None => return Err(fatal("The application did not say where to return to.")),
    };
    match validate_request(params) {
        Ok(scope) => Ok(ValidAuthorization {
            code_challenge: params.code_challenge.clone().unwrap_or_default(),
            state: params.state.clone(),
            client,
            redirect_uri,
            scope,
        }),
        Err(error) => Err(AuthorizeError::Redirect {
            location: error_redirect(&redirect_uri, &error, params.state.as_deref()),
        }),
    }
}

fn error_redirect(redirect_uri: &str, error: &RedirectableError, state: Option<&str>) -> String {
    let mut params = vec![
        ("error", error.error),
        ("error_description", error.description.as_str()),
    ];
    if let Some(state) = state {
        params.push(("state", state));
    }
    redirect_with(redirect_uri, &params)
}

/// Issues a single-use code and returns where to send the browser.
pub async fn approve(
    state: &AppState,
    user_id: Uuid,
    authorization: &ValidAuthorization,
) -> ApiResult<String> {
    let active = connections::count_active(&state.db, user_id).await?;
    if active >= MAX_ACTIVE_CONNECTIONS as i64 {
        return Err(ApiError::Conflict {
            code: "too_many_connections",
            message: "Revoke an unused connection in Settings before connecting another agent."
                .into(),
        });
    }
    let code = generate_token(CODE_PREFIX).map_err(ApiError::internal)?;
    repository::delete_stale_codes(&state.db).await?;
    repository::insert_code(
        &state.db,
        &NewCode {
            code_hash: &hash_token(&code),
            client_id: authorization.client.id,
            user_id,
            redirect_uri: &authorization.redirect_uri,
            code_challenge: &authorization.code_challenge,
            scope: &authorization.scope,
            expires_at: Utc::now() + CODE_LIFETIME,
        },
    )
    .await?;
    let mut params = vec![("code", code.as_str())];
    if let Some(client_state) = authorization.state.as_deref() {
        params.push(("state", client_state));
    }
    Ok(redirect_with(&authorization.redirect_uri, &params))
}

pub fn deny(authorization: &ValidAuthorization) -> String {
    error_redirect(
        &authorization.redirect_uri,
        &RedirectableError {
            error: "access_denied",
            description: "the user denied access".into(),
        },
        authorization.state.as_deref(),
    )
}

pub async fn register_client(
    state: &AppState,
    client_name: Option<&str>,
    redirect_uris: &[String],
) -> Result<OAuthClient, OAuthError> {
    validate_redirect_uris(redirect_uris)
        .map_err(|message| OAuthError::bad_request("invalid_redirect_uri", message))?;
    let name: String = client_name
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or("MCP client")
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_CLIENT_NAME)
        .collect();
    Ok(repository::insert_client(&state.db, &name, redirect_uris).await?)
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: &'static str,
    pub expires_in: i64,
    pub refresh_token: String,
    pub scope: &'static str,
}

struct IssuedTokens {
    access: String,
    refresh: String,
    access_hash: Vec<u8>,
    refresh_hash: Vec<u8>,
}

impl IssuedTokens {
    fn generate() -> Result<Self, OAuthError> {
        let access = generate_token(MCP_TOKEN_PREFIX).map_err(OAuthError::server_error)?;
        let refresh = generate_token(REFRESH_TOKEN_PREFIX).map_err(OAuthError::server_error)?;
        Ok(Self {
            access_hash: hash_token(&access),
            refresh_hash: hash_token(&refresh),
            access,
            refresh,
        })
    }

    fn record<'a>(&'a self, hint: &'a str) -> OAuthTokens<'a> {
        let now = Utc::now();
        OAuthTokens {
            access_hash: &self.access_hash,
            access_hint: hint,
            refresh_hash: &self.refresh_hash,
            access_expires_at: now + ACCESS_TOKEN_LIFETIME,
            expires_at: now + GRANT_LIFETIME,
        }
    }

    fn into_response(self) -> TokenResponse {
        TokenResponse {
            access_token: self.access,
            token_type: "Bearer",
            expires_in: ACCESS_TOKEN_LIFETIME.num_seconds(),
            refresh_token: self.refresh,
            scope: super::protocol::SCOPE,
        }
    }
}

async fn authenticate_client(
    state: &AppState,
    client_id: Option<&str>,
) -> Result<OAuthClient, OAuthError> {
    let invalid = || OAuthError::new(StatusCode::UNAUTHORIZED, "invalid_client", "unknown client");
    let id = client_id
        .and_then(|id| Uuid::parse_str(id).ok())
        .ok_or_else(invalid)?;
    repository::find_client(&state.db, id)
        .await?
        .ok_or_else(invalid)
}

pub async fn exchange_code(
    state: &AppState,
    client_id: Option<&str>,
    code: &str,
    redirect_uri: Option<&str>,
    code_verifier: &str,
) -> Result<TokenResponse, OAuthError> {
    let client = authenticate_client(state, client_id).await?;
    let code_hash = hash_token(code);
    let Some(grant) = repository::consume_code(&state.db, &code_hash).await? else {
        // A replayed code suggests it leaked; revoke what it was exchanged for
        // (RFC 6749 §4.1.2).
        if let Some(connection_id) =
            repository::replayed_code_connection(&state.db, &code_hash).await?
        {
            connections::revoke_by_id(&state.db, connection_id).await?;
        }
        return Err(OAuthError::invalid_grant());
    };
    let redirect_ok = redirect_uri.is_none_or(|uri| uri == grant.redirect_uri);
    if grant.client_id != client.id || !redirect_ok {
        return Err(OAuthError::invalid_grant());
    }
    if !verify_pkce(code_verifier, &grant.code_challenge) {
        return Err(OAuthError::bad_request(
            "invalid_grant",
            "code_verifier does not match the code challenge",
        ));
    }

    let tokens = IssuedTokens::generate()?;
    let hint = token_hint(&tokens.access);
    let label: String = client.client_name.chars().take(MAX_LABEL).collect();
    let mut tx = state.db.begin().await?;
    connections::revoke_oauth_client_grants(&mut *tx, grant.user_id, client.id).await?;
    let connection_id = connections::insert_oauth(
        &mut *tx,
        grant.user_id,
        client.id,
        &label,
        &tokens.record(&hint),
    )
    .await?;
    repository::link_code_connection(&mut *tx, grant.id, connection_id).await?;
    tx.commit().await?;
    Ok(tokens.into_response())
}

pub async fn refresh(
    state: &AppState,
    client_id: Option<&str>,
    refresh_token: &str,
) -> Result<TokenResponse, OAuthError> {
    let client = authenticate_client(state, client_id).await?;
    let tokens = IssuedTokens::generate()?;
    let hint = token_hint(&tokens.access);
    connections::rotate_oauth(
        &state.db,
        client.id,
        &hash_token(refresh_token),
        &tokens.record(&hint),
    )
    .await?
    .ok_or_else(OAuthError::invalid_grant)?;
    Ok(tokens.into_response())
}
