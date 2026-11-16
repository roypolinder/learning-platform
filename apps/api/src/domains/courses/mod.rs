pub mod catalog;
pub mod dto;
mod handlers;
pub mod repository;
pub mod sync;

use axum::Router;
use axum::routing::get;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/courses", get(handlers::list))
        .route("/courses/{course_id}", get(handlers::detail))
        .route("/course-assets/{name}", get(handlers::asset))
}
