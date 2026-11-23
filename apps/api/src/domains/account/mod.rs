pub mod domain;
pub mod dto;
mod handlers;
pub mod repository;
pub mod service;

use axum::Router;
use axum::routing::{delete, post};

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/password/forgot", post(handlers::forgot_password))
        .route("/auth/password/reset", post(handlers::reset_password))
        .route("/auth/email/confirm", post(handlers::confirm_email))
        .route(
            "/users/me/email/verification",
            post(handlers::resend_verification),
        )
        .route("/users/me/email", post(handlers::change_email))
        .route(
            "/users/me/email/pending",
            delete(handlers::cancel_email_change),
        )
        .route("/users/me/password", post(handlers::change_password))
        .route("/users/me/delete", post(handlers::delete_account))
}
