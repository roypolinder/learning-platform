use std::net::SocketAddr;

use axum::Form;
use axum::Json;
use axum::extract::rejection::{FormRejection, JsonRejection, QueryRejection};
use axum::extract::{ConnectInfo, Query, RawQuery, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use url::Url;

use super::protocol::{self, AuthorizationParams, SCOPE, request_origin};
use super::service::{self, AuthorizeError, OAuthError, ValidAuthorization, no_store};
use crate::error::{ApiError, ApiResult};
use crate::http::auth::CurrentUser;
use crate::http::client_ip::client_ip;
use crate::http::json::ApiJson;
use crate::state::AppState;

/// RFC 9728. Advertised in the `/mcp` 401 challenge.
pub async fn protected_resource_metadata(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<Value> {
    let origin = request_origin(&state.config, &headers);
    Json(json!({
        "resource": format!("{origin}/mcp"),
        "authorization_servers": [origin],
        "scopes_supported": [SCOPE],
        "bearer_methods_supported": ["header"],
        "resource_name": "Learning Platform"
    }))
}

/// RFC 8414.
pub async fn authorization_server_metadata(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<Value> {
    let origin = request_origin(&state.config, &headers);
    Json(json!({
        "issuer": origin,
        "authorization_endpoint": format!("{origin}/oauth/authorize"),
        "token_endpoint": format!("{origin}/oauth/token"),
        "registration_endpoint": format!("{origin}/oauth/register"),
        "scopes_supported": [SCOPE],
        "response_types_supported": ["code"],
        "response_modes_supported": ["query"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "token_endpoint_auth_methods_supported": ["none"],
        "code_challenge_methods_supported": ["S256"]
    }))
}

/// Validates the request, then hands the browser to the web app's consent page,
/// which signs the user in if needed.
pub async fn authorize(
    State(state): State<AppState>,
    params: Result<Query<AuthorizationParams>, QueryRejection>,
    RawQuery(query): RawQuery,
) -> Response {
    let Ok(Query(params)) = params else {
        return authorize_error_page("The authorization request is malformed.");
    };
    match service::validate_authorization(&state, &params).await {
        Ok(_) => Redirect::to(&format!(
            "{}/connect/authorize?{}",
            state.config.web_base_url,
            query.unwrap_or_default()
        ))
        .into_response(),
        Err(AuthorizeError::Redirect { location }) => Redirect::to(&location).into_response(),
        Err(AuthorizeError::Fatal(message)) => authorize_error_page(&message),
    }
}

fn authorize_error_page(message: &str) -> Response {
    let escaped = message
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    (
        StatusCode::BAD_REQUEST,
        Html(format!(
            "<!doctype html><meta charset=\"utf-8\"><title>Cannot connect</title>\
             <body style=\"font-family:system-ui;max-width:32rem;margin:4rem auto\">\
             <h1>Cannot connect this application</h1><p>{escaped}</p>\
             <p>Restart the sign-in from your editor.</p></body>"
        )),
    )
        .into_response()
}

#[derive(Debug, Deserialize)]
pub struct RegistrationRequest {
    #[serde(default)]
    redirect_uris: Vec<String>,
    client_name: Option<String>,
    grant_types: Option<Vec<String>>,
    response_types: Option<Vec<String>>,
}

/// RFC 7591 dynamic client registration. Unknown metadata is ignored, and every
/// client is registered as a public client.
pub async fn register(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Result<Json<RegistrationRequest>, JsonRejection>,
) -> Result<Response, OAuthError> {
    let ip = client_ip(&state.config, peer, &headers);
    state.auth_limiter.check(&format!("oauth-register:{ip}"))?;
    let Json(request) = body.map_err(|rejection| {
        OAuthError::bad_request("invalid_client_metadata", rejection.body_text())
    })?;
    let grant_types_ok = request.grant_types.as_ref().is_none_or(|types| {
        types
            .iter()
            .all(|t| t == "authorization_code" || t == "refresh_token")
    });
    let response_types_ok = request
        .response_types
        .as_ref()
        .is_none_or(|types| types.iter().all(|t| t == "code"));
    if !grant_types_ok || !response_types_ok {
        return Err(OAuthError::bad_request(
            "invalid_client_metadata",
            "only the authorization_code and refresh_token grants are supported",
        ));
    }
    let client = service::register_client(
        &state,
        request.client_name.as_deref(),
        &request.redirect_uris,
    )
    .await?;
    Ok(no_store(
        StatusCode::CREATED,
        Json(json!({
            "client_id": client.id.to_string(),
            "client_id_issued_at": client.created_at.timestamp(),
            "client_name": client.client_name,
            "redirect_uris": client.redirect_uris,
            "grant_types": ["authorization_code", "refresh_token"],
            "response_types": ["code"],
            "token_endpoint_auth_method": "none",
            "scope": SCOPE
        })),
    ))
}

#[derive(Debug, Deserialize)]
pub struct TokenRequest {
    grant_type: Option<String>,
    client_id: Option<String>,
    code: Option<String>,
    redirect_uri: Option<String>,
    code_verifier: Option<String>,
    refresh_token: Option<String>,
    resource: Option<String>,
    scope: Option<String>,
}

pub async fn token(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    form: Result<Form<TokenRequest>, FormRejection>,
) -> Result<Response, OAuthError> {
    let ip = client_ip(&state.config, peer, &headers);
    state.mcp_limiter.check(&format!("oauth-token:{ip}"))?;
    let Form(request) = form
        .map_err(|rejection| OAuthError::bad_request("invalid_request", rejection.body_text()))?;
    if request
        .resource
        .as_deref()
        .is_some_and(|resource| !protocol::is_mcp_resource(resource))
    {
        return Err(OAuthError::bad_request(
            "invalid_target",
            "resource must be this platform's MCP endpoint",
        ));
    }
    if request.scope.is_some() && protocol::granted_scope(request.scope.as_deref()).is_none() {
        return Err(OAuthError::bad_request(
            "invalid_scope",
            format!("only `{SCOPE}` is supported"),
        ));
    }
    let required = |value: Option<String>, name: &str| {
        value.filter(|value| !value.is_empty()).ok_or_else(|| {
            OAuthError::bad_request("invalid_request", format!("{name} is required"))
        })
    };
    let tokens = match request.grant_type.as_deref() {
        Some("authorization_code") => {
            let code = required(request.code, "code")?;
            let verifier = required(request.code_verifier, "code_verifier")?;
            service::exchange_code(
                &state,
                request.client_id.as_deref(),
                &code,
                request.redirect_uri.as_deref(),
                &verifier,
            )
            .await?
        }
        Some("refresh_token") => {
            let refresh_token = required(request.refresh_token, "refresh_token")?;
            service::refresh(&state, request.client_id.as_deref(), &refresh_token).await?
        }
        _ => {
            return Err(OAuthError::bad_request(
                "unsupported_grant_type",
                "grant_type must be authorization_code or refresh_token",
            ));
        }
    };
    Ok(no_store(StatusCode::OK, Json(tokens)))
}

#[derive(Serialize)]
pub struct AuthorizationPreview {
    client_name: String,
    /// Where the browser returns to, so the user can spot an unexpected destination.
    redirect_target: String,
    scope: String,
}

pub async fn preview(
    State(state): State<AppState>,
    _user: CurrentUser,
    Query(params): Query<AuthorizationParams>,
) -> ApiResult<Json<AuthorizationPreview>> {
    let authorization = consent_request(&state, &params).await?;
    Ok(Json(AuthorizationPreview {
        redirect_target: redirect_target(&authorization.redirect_uri),
        client_name: authorization.client.client_name,
        scope: authorization.scope,
    }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionRequest {
    params: AuthorizationParams,
    approve: bool,
}

#[derive(Serialize)]
pub struct DecisionResponse {
    redirect_to: String,
}

pub async fn decide(
    State(state): State<AppState>,
    user: CurrentUser,
    ApiJson(request): ApiJson<DecisionRequest>,
) -> ApiResult<Json<DecisionResponse>> {
    let authorization = match service::validate_authorization(&state, &request.params).await {
        Ok(authorization) => authorization,
        Err(AuthorizeError::Redirect { location }) => {
            return Ok(Json(DecisionResponse {
                redirect_to: location,
            }));
        }
        Err(AuthorizeError::Fatal(message)) => return Err(invalid_request(message)),
    };
    let redirect_to = if request.approve {
        service::approve(&state, user.id, &authorization).await?
    } else {
        service::deny(&authorization)
    };
    Ok(Json(DecisionResponse { redirect_to }))
}

async fn consent_request(
    state: &AppState,
    params: &AuthorizationParams,
) -> ApiResult<ValidAuthorization> {
    match service::validate_authorization(state, params).await {
        Ok(authorization) => Ok(authorization),
        Err(AuthorizeError::Fatal(message)) => Err(invalid_request(message)),
        Err(AuthorizeError::Redirect { .. }) => Err(invalid_request(
            "The application sent an invalid sign-in request. Restart the sign-in from your editor."
                .into(),
        )),
    }
}

fn invalid_request(message: String) -> ApiError {
    ApiError::BadRequest {
        code: "invalid_authorization_request",
        message,
    }
}

fn redirect_target(redirect_uri: &str) -> String {
    Url::parse(redirect_uri)
        .ok()
        .map(|url| match url.host_str() {
            Some(host) => host.to_owned(),
            None => format!("{}:", url.scheme()),
        })
        .unwrap_or_default()
}
