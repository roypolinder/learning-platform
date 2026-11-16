pub mod domain;
mod handlers;
pub mod repository;
pub mod service;

use axum::Router;
use axum::routing::{delete, get, post};

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/integrations", get(handlers::list))
        .route("/integrations/mcp", post(handlers::create_mcp))
        .route("/integrations/{connection_id}", delete(handlers::revoke))
}
