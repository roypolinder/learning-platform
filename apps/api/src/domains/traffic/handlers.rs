use std::io::ErrorKind;
use std::time::SystemTime;

use axum::Json;
use axum::extract::State;
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::error::{ApiError, ApiResult};
use crate::http::auth::SuperAdmin;
use crate::state::AppState;

/// Generated reports are a few MB at most; refuse anything unexpectedly large.
const MAX_REPORT_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Debug, Serialize)]
pub struct TrafficReport {
    pub generated_at: DateTime<Utc>,
    /// Self-contained HTML document. Render it in a sandboxed iframe, never inline.
    pub html: String,
}

pub async fn report(
    State(state): State<AppState>,
    _: SuperAdmin,
) -> ApiResult<Json<TrafficReport>> {
    let path = state
        .config
        .traffic_report_path
        .as_ref()
        .ok_or(ApiError::NotFound("traffic report"))?;
    let internal = |error: std::io::Error| ApiError::Internal(format!("traffic report: {error}"));
    let metadata = match tokio::fs::metadata(path).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Err(ApiError::NotFound("traffic report"));
        }
        Err(error) => return Err(internal(error)),
    };
    if metadata.len() > MAX_REPORT_BYTES {
        return Err(ApiError::Unavailable(
            "the traffic report is too large".into(),
        ));
    }
    let html = tokio::fs::read_to_string(path).await.map_err(internal)?;
    let generated_at = metadata
        .modified()
        .unwrap_or_else(|_| SystemTime::now())
        .into();
    Ok(Json(TrafficReport { generated_at, html }))
}
