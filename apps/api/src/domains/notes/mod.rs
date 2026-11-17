//! Personal notes a learner keeps per course next to the glossary.

mod handlers;
mod repository;

use axum::Router;
use axum::routing::{get, patch};

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/notes", get(handlers::list).post(handlers::create))
        .route(
            "/notes/{id}",
            patch(handlers::update).delete(handlers::remove),
        )
}
