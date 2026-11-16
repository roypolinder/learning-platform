use axum::extract::rejection::JsonRejection;
use axum::extract::{FromRequest, Request};
use serde::de::DeserializeOwned;

use crate::error::ApiError;

/// `Json` with rejections converted into the API's structured error format.
pub struct ApiJson<T>(pub T);

impl<S, T> FromRequest<S> for ApiJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        match axum::Json::<T>::from_request(request, state).await {
            Ok(axum::Json(value)) => Ok(Self(value)),
            Err(rejection) => Err(ApiError::BadRequest {
                code: match rejection {
                    JsonRejection::MissingJsonContentType(_) => "unsupported_media_type",
                    JsonRejection::JsonSyntaxError(_) => "malformed_json",
                    _ => "invalid_body",
                },
                message: rejection.body_text(),
            }),
        }
    }
}
