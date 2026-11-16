mod handlers;
pub mod repository;

use axum::Router;
use axum::routing::get;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/extensions", get(handlers::list))
        .route("/extensions/step-types", get(handlers::step_types))
}
