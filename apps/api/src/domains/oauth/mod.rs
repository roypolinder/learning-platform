//! OAuth 2.1 authorization server for MCP clients (MCP authorization spec): metadata
//! discovery, dynamic client registration, authorization code + PKCE, refresh token
//! rotation. Consent happens in the web app with the user's normal browser session;
//! issued grants are MCP connections, so they can be listed and revoked like tokens.

mod handlers;
pub mod protocol;
pub mod repository;
pub mod service;

use axum::Router;
use axum::routing::{get, post};

use crate::state::AppState;

/// Endpoints called by MCP clients. They sit outside `/api/v1` because clients
/// cannot send the CSRF header and some paths are fixed by the specs.
pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/.well-known/oauth-protected-resource",
            get(handlers::protected_resource_metadata),
        )
        .route(
            "/.well-known/oauth-protected-resource/mcp",
            get(handlers::protected_resource_metadata),
        )
        .route(
            "/.well-known/oauth-authorization-server",
            get(handlers::authorization_server_metadata),
        )
        .route("/oauth/authorize", get(handlers::authorize))
        .route("/oauth/register", post(handlers::register))
        .route("/oauth/token", post(handlers::token))
}

/// Consent API used by the web app's `/connect/authorize` page.
pub fn routes() -> Router<AppState> {
    Router::new().route(
        "/oauth/authorization",
        get(handlers::preview).post(handlers::decide),
    )
}
