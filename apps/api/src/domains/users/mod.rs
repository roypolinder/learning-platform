pub mod domain;
pub mod dto;
mod handlers;
pub mod repository;

use axum::Router;
use axum::routing::get;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/users/me", get(handlers::me).patch(handlers::update_me))
        .route("/auth/session", get(handlers::session))
}
