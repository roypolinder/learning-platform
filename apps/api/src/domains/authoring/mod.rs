mod handlers;
pub mod service;

use axum::Router;
use axum::routing::{get, post};

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/admin/courses",
            get(handlers::list).post(handlers::publish),
        )
        .route("/admin/courses/validate", post(handlers::validate))
        .route("/admin/courses/{course_id}", get(handlers::detail))
        .route("/admin/authoring-guide", get(handlers::guide))
}
