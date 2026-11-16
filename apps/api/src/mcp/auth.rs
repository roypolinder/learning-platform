use axum::extract::{Request, State};
use axum::http::header::{AUTHORIZATION, WWW_AUTHENTICATE};
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::domains::integrations::repository;
use crate::domains::oauth::protocol::{SCOPE, request_origin};
use crate::error::ApiError;
use crate::security::{MCP_TOKEN_PREFIX, hash_token};
use crate::state::AppState;

pub async fn authenticate(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let resource_metadata = format!(
        "{}/.well-known/oauth-protected-resource/mcp",
        request_origin(&state.config, request.headers())
    );
    let presented = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::to_owned);
    let Some(token) = presented else {
        return unauthorized(&resource_metadata, None);
    };
    let principal = match token.starts_with(MCP_TOKEN_PREFIX) {
        true => match repository::authenticate(&state.db, &hash_token(&token)).await {
            Ok(principal) => principal,
            Err(error) => return ApiError::from(error).into_response(),
        },
        false => None,
    };
    let Some(principal) = principal else {
        return unauthorized(
            &resource_metadata,
            Some("token is invalid, expired or revoked"),
        );
    };
    if let Err(error) = state
        .mcp_limiter
        .check(&principal.connection_id.to_string())
    {
        return error.into_response();
    }
    request.extensions_mut().insert(principal);
    next.run(request).await
}

/// RFC 6750 challenge pointing at the protected resource metadata (RFC 9728), from
/// which MCP clients discover the OAuth endpoints and start the sign-in flow. Without
/// a presented token no `error` is included, as RFC 6750 §3.1 requires.
fn unauthorized(resource_metadata: &str, error: Option<&str>) -> Response {
    let mut challenge =
        format!("Bearer resource_metadata=\"{resource_metadata}\", scope=\"{SCOPE}\"");
    if let Some(reason) = error {
        challenge.push_str(&format!(
            ", error=\"invalid_token\", error_description=\"{reason}\""
        ));
    }
    let message = error.unwrap_or("sign in to the learning platform to use this server");
    let mut response = (
        StatusCode::UNAUTHORIZED,
        axum::Json(serde_json::json!({ "error": { "code": "unauthorized", "message": message } })),
    )
        .into_response();
    if let Ok(value) = HeaderValue::from_str(&challenge) {
        response.headers_mut().insert(WWW_AUTHENTICATE, value);
    }
    response
}
