//! Cookie-authenticated state changes must carry a custom header. Browsers cannot
//! attach custom headers to cross-site form posts, and cross-origin scripts would
//! need a CORS preflight that this API never grants.

use axum::extract::Request;
use axum::http::{Method, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::error::ApiError;

pub const CLIENT_HEADER: &str = "x-requested-with";

pub async fn require_client_header(request: Request, next: Next) -> Response {
    let safe = matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    );
    let bearer = request.headers().contains_key(header::AUTHORIZATION);
    if safe || bearer || request.headers().contains_key(CLIENT_HEADER) {
        next.run(request).await
    } else {
        ApiError::BadRequest {
            code: "missing_client_header",
            message: format!("state-changing requests must send the `{CLIENT_HEADER}` header"),
        }
        .into_response()
    }
}
