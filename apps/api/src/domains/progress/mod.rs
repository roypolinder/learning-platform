mod handlers;
pub mod service;

use axum::Router;
use axum::routing::get;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/progress/overview", get(handlers::overview))
}
