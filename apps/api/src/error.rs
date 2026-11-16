use axum::Json;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use lp_core::ExtensionError;
use lp_course_engine::EngineError;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{message}")]
    BadRequest { code: &'static str, message: String },
    #[error("request validation failed")]
    Validation(Vec<FieldError>),
    #[error("authentication required")]
    Unauthorized,
    #[error("you do not have permission to do this")]
    Forbidden,
    #[error("{0} not found")]
    NotFound(&'static str),
    #[error("{message}")]
    Conflict { code: &'static str, message: String },
    #[error("{message}")]
    Unprocessable {
        code: &'static str,
        message: String,
        details: Option<Value>,
    },
    #[error("too many requests")]
    RateLimited { retry_after_seconds: u64 },
    #[error("{0}")]
    Unavailable(String),
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct FieldError {
    pub field: &'static str,
    pub message: String,
}

impl FieldError {
    pub fn new(field: &'static str, message: impl Into<String>) -> Self {
        Self {
            field,
            message: message.into(),
        }
    }
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    error: ErrorPayload<'a>,
}

#[derive(Serialize)]
struct ErrorPayload<'a> {
    code: &'a str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<Value>,
}

impl ApiError {
    pub fn internal(error: impl std::fmt::Display) -> Self {
        ApiError::Internal(error.to_string())
    }

    fn status_and_code(&self) -> (StatusCode, &'static str) {
        match self {
            ApiError::BadRequest { code, .. } => (StatusCode::BAD_REQUEST, code),
            ApiError::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, "validation_failed"),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, "forbidden"),
            ApiError::NotFound(_) => (StatusCode::NOT_FOUND, "not_found"),
            ApiError::Conflict { code, .. } => (StatusCode::CONFLICT, code),
            ApiError::Unprocessable { code, .. } => (StatusCode::UNPROCESSABLE_ENTITY, code),
            ApiError::RateLimited { .. } => (StatusCode::TOO_MANY_REQUESTS, "rate_limited"),
            ApiError::Unavailable(_) => (StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
            ApiError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = self.status_and_code();
        let (message, details) = match &self {
            ApiError::Internal(detail) => {
                tracing::error!(error = %detail, "internal error");
                ("Something went wrong on our side.".to_owned(), None)
            }
            ApiError::Validation(fields) => (self.to_string(), serde_json::to_value(fields).ok()),
            ApiError::Unprocessable { details, .. } => (self.to_string(), details.clone()),
            _ => (self.to_string(), None),
        };
        let mut response = (
            status,
            Json(ErrorBody {
                error: ErrorPayload {
                    code,
                    message,
                    details,
                },
            }),
        )
            .into_response();
        match self {
            ApiError::RateLimited {
                retry_after_seconds,
            } => {
                if let Ok(value) = HeaderValue::from_str(&retry_after_seconds.to_string()) {
                    response.headers_mut().insert(header::RETRY_AFTER, value);
                }
            }
            ApiError::Unauthorized => {
                response
                    .headers_mut()
                    .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
            }
            _ => {}
        }
        response
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> Self {
        ApiError::Internal(error.to_string())
    }
}

impl From<EngineError> for ApiError {
    fn from(error: EngineError) -> Self {
        match error {
            EngineError::UnknownStep(_) => ApiError::NotFound("step"),
            EngineError::StepNotActionable { .. } => ApiError::Conflict {
                code: "step_not_actionable",
                message: error.to_string(),
            },
            EngineError::UnsupportedAction { .. } => ApiError::BadRequest {
                code: "unsupported_action",
                message: error.to_string(),
            },
            EngineError::MissingExtension(_) => ApiError::Internal(error.to_string()),
            EngineError::Extension(extension) => extension.into(),
        }
    }
}

impl From<ExtensionError> for ApiError {
    fn from(error: ExtensionError) -> Self {
        match error {
            ExtensionError::InvalidSubmission(message) => ApiError::Unprocessable {
                code: "invalid_submission",
                message,
                details: None,
            },
            ExtensionError::Unsupported(message) => ApiError::BadRequest {
                code: "unsupported",
                message,
            },
            ExtensionError::Unavailable(message) => ApiError::Unavailable(message),
        }
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
