//! The learner's own context: interface language and the project (website theme and
//! stack) they build throughout the course track. Courses are rendered for it.

pub mod domain;
mod handlers;
pub mod repository;

use axum::Router;
use axum::routing::get;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/themes", get(handlers::themes))
        .route(
            "/project",
            get(handlers::project).post(handlers::create_project),
        )
        .route("/project/data", get(handlers::project_data))
}
