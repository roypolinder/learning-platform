mod handlers;

use axum::Router;
use axum::routing::{get, post};

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/admin/logs", get(handlers::list))
        .route("/admin/mail/test", post(handlers::send_test_email))
}
