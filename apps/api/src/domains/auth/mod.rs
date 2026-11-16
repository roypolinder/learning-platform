pub mod dto;
mod handlers;
pub mod repository;
pub mod service;

use axum::Router;
use axum::routing::post;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(handlers::register))
        .route("/auth/login", post(handlers::login))
        .route("/auth/logout", post(handlers::logout))
}
