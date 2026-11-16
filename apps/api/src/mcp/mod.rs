//! MCP gateway. Agents authenticate with a per-user bearer token, obtained through the
//! OAuth flow in `domains::oauth` or created manually; every tool
//! is scoped to that user's sessions, and tool calls are reported to the course
//! engine as `mcp.tool_called` events. The gateway exposes no filesystem access
//! or command execution — agents send evidence, the platform validates it.

mod auth;
mod server;

use axum::Router;
use axum::middleware;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use tokio_util::sync::CancellationToken;

use crate::state::AppState;

pub use server::LearningPlatformServer;

pub fn router(state: AppState, shutdown: CancellationToken) -> Router {
    let config = StreamableHttpServerConfig::default()
        .with_allowed_hosts(state.config.mcp_allowed_hosts.clone())
        // Agents don't send Origin; rejecting browser origins blocks DNS-rebinding
        // and drive-by requests from web pages.
        .enforce_origin_validation()
        .with_max_request_body_bytes(2 * 1024 * 1024)
        .with_cancellation_token(shutdown);
    let factory_state = state.clone();
    let service: StreamableHttpService<LearningPlatformServer, LocalSessionManager> =
        StreamableHttpService::new(
            move || Ok(LearningPlatformServer::new(factory_state.clone())),
            Default::default(),
            config,
        );
    Router::new()
        .nest_service("/mcp", service)
        // route_layer: only /mcp is protected; unknown paths keep returning 404.
        .route_layer(middleware::from_fn_with_state(state, auth::authenticate))
}
