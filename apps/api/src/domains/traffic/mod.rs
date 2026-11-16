mod handlers;

use axum::Router;
use axum::routing::get;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/admin/traffic-report", get(handlers::report))
}
