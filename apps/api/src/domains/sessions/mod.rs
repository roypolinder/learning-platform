pub mod domain;
pub mod dto;
mod handlers;
pub mod repository;
pub mod service;
mod stream;

use axum::Router;
use axum::routing::{get, post};

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/courses/{course_id}/sessions", post(handlers::start))
        .route("/sessions", get(handlers::list))
        .route("/sessions/{session_id}", get(handlers::detail))
        .route("/sessions/{session_id}/progress", get(handlers::progress))
        .route("/sessions/{session_id}/events", get(handlers::events))
        .route("/sessions/{session_id}/refresh", post(handlers::refresh))
        .route(
            "/sessions/{session_id}/steps/{step_id}/open",
            post(handlers::open_step),
        )
        .route(
            "/sessions/{session_id}/steps/{step_id}/complete",
            post(handlers::complete_step),
        )
        .route(
            "/sessions/{session_id}/steps/{step_id}/submissions",
            post(handlers::submit),
        )
}

/// Long-lived streams are routed separately so request timeouts don't cut them.
pub fn stream_routes() -> Router<AppState> {
    Router::new().route("/sessions/{session_id}/stream", get(stream::stream))
}
