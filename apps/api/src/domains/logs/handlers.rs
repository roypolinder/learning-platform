//! The super admin log: warnings, errors and audit events stored by `observability`.

use axum::Json;
use axum::extract::{Query, State};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::http::auth::SuperAdmin;
use crate::http::pagination::{Page, PageParams};
use crate::mail::{self, templates::Templates};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct LogQuery {
    /// `error`, `warn` or `info`.
    pub level: Option<String>,
    /// e.g. `mail` or `account`.
    pub category: Option<String>,
    /// Text in the message or fields, such as an email address.
    pub q: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct LogEntry {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub level: String,
    pub target: String,
    pub category: Option<String>,
    pub message: String,
    pub fields: Value,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct LogCounts {
    pub errors_24h: i64,
    pub warnings_24h: i64,
}

#[derive(Debug, Serialize)]
pub struct LogsResponse {
    pub counts: LogCounts,
    pub retention_days: i32,
    pub logs: Page<LogEntry>,
}

const FILTER: &str = "($1::text IS NULL OR level = $1)
     AND ($2::text IS NULL OR category = $2)
     AND ($3::text IS NULL OR message ILIKE $3 OR fields::text ILIKE $3 OR target ILIKE $3)";

pub async fn list(
    State(state): State<AppState>,
    _: SuperAdmin,
    Query(query): Query<LogQuery>,
) -> ApiResult<Json<LogsResponse>> {
    let params = PageParams {
        page: query.page,
        per_page: query.per_page,
    };
    let level = query
        .level
        .filter(|level| ["error", "warn", "info"].contains(&level.as_str()));
    let category = query.category.filter(|category| !category.is_empty());
    let pattern = query
        .q
        .as_deref()
        .map(str::trim)
        .filter(|q| !q.is_empty())
        .map(|q| {
            let escaped = q
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_");
            format!("%{escaped}%")
        });

    let total: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM system_logs WHERE {FILTER}"
    )))
    .bind(&level)
    .bind(&category)
    .bind(&pattern)
    .fetch_one(&state.db)
    .await?;
    let items = sqlx::query_as::<_, LogEntry>(sqlx::AssertSqlSafe(format!(
        "SELECT id, created_at, level, target, category, message, fields FROM system_logs
         WHERE {FILTER} ORDER BY created_at DESC, id DESC LIMIT $4 OFFSET $5"
    )))
    .bind(&level)
    .bind(&category)
    .bind(&pattern)
    .bind(params.limit())
    .bind(params.offset())
    .fetch_all(&state.db)
    .await?;
    let counts = sqlx::query_as::<_, LogCounts>(
        "SELECT count(*) FILTER (WHERE level = 'error') AS errors_24h,
                count(*) FILTER (WHERE level = 'warn') AS warnings_24h
         FROM system_logs WHERE created_at > now() - interval '24 hours'",
    )
    .fetch_one(&state.db)
    .await?;

    Ok(Json(LogsResponse {
        counts,
        retention_days: crate::observability::RETENTION_DAYS,
        logs: Page::new(items, params, total.max(0) as u64),
    }))
}

#[derive(Debug, Serialize)]
pub struct TestEmailResult {
    pub to: String,
    /// The mail server's reply.
    pub response: String,
}

/// Sends a test email to the signed-in admin and waits for the mail server's answer, so
/// configuration problems show up right away instead of only in the log.
pub async fn send_test_email(
    State(state): State<AppState>,
    SuperAdmin(admin): SuperAdmin,
) -> ApiResult<Json<TestEmailResult>> {
    state
        .mail_limiter
        .check(&format!("mail:{}", admin.email.to_lowercase()))?;
    let email = Templates::new("en", &state.config.web_base_url)
        .test_email(&admin.email, &admin.display_name);
    match mail::send_logged(state.mailer.as_ref(), email).await {
        Ok(response) => Ok(Json(TestEmailResult {
            to: admin.email,
            response,
        })),
        Err(error) => Err(ApiError::Unprocessable {
            code: "mail_failed",
            message: error.to_string(),
            details: None,
        }),
    }
}
