//! Super admin overview of accounts, for spotting fake or automated sign-ups.

use axum::Json;
use axum::extract::{Query, State};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ApiResult;
use crate::http::auth::SuperAdmin;
use crate::http::pagination::{Page, PageParams};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct UserListQuery {
    /// Part of the email address or name.
    pub q: Option<String>,
    /// `verified` or `unverified`; anything else lists everyone.
    pub status: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AdminUserRow {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub locale: String,
    pub created_at: DateTime<Utc>,
    pub email_verified_at: Option<DateTime<Utc>>,
    /// Newest browser sign-in (registration counts as one).
    pub last_sign_in_at: Option<DateTime<Utc>>,
    pub sign_ins: i64,
    pub courses_started: i64,
    pub courses_completed: i64,
    pub theme_id: Option<String>,
    pub agent_connected: bool,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SignupDay {
    pub day: NaiveDate,
    pub count: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct UserSummary {
    pub total: i64,
    pub verified: i64,
    pub last_24_hours: i64,
    pub last_7_days: i64,
    pub last_30_days: i64,
    #[sqlx(skip)]
    pub signups_per_day: Vec<SignupDay>,
}

#[derive(Debug, Serialize)]
pub struct AdminUsersResponse {
    pub summary: UserSummary,
    pub users: Page<AdminUserRow>,
}

const FILTER: &str = "($1::text IS NULL OR u.email ILIKE $1 OR u.display_name ILIKE $1)
     AND ($2::text IS NULL
          OR ($2 = 'verified' AND u.email_verified_at IS NOT NULL)
          OR ($2 = 'unverified' AND u.email_verified_at IS NULL))";

pub async fn list(
    State(state): State<AppState>,
    _: SuperAdmin,
    Query(query): Query<UserListQuery>,
) -> ApiResult<Json<AdminUsersResponse>> {
    let params = PageParams {
        page: query.page,
        per_page: query.per_page,
    };
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
    let status = query
        .status
        .filter(|status| status == "verified" || status == "unverified");

    let total: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM users u WHERE {FILTER}"
    )))
    .bind(&pattern)
    .bind(&status)
    .fetch_one(&state.db)
    .await?;

    let rows = sqlx::query_as::<_, AdminUserRow>(sqlx::AssertSqlSafe(format!(
        "SELECT u.id, u.email, u.display_name, u.role, u.locale, u.created_at, u.email_verified_at,
                s.last_sign_in_at, COALESCE(s.sign_ins, 0) AS sign_ins,
                COALESCE(c.started, 0) AS courses_started,
                COALESCE(c.completed, 0) AS courses_completed,
                p.theme_id,
                EXISTS (SELECT 1 FROM integration_connections i
                        WHERE i.user_id = u.id AND i.status = 'connected'
                          AND i.revoked_at IS NULL) AS agent_connected
         FROM users u
         LEFT JOIN LATERAL (
             SELECT max(created_at) AS last_sign_in_at, count(*) AS sign_ins
             FROM auth_sessions WHERE user_id = u.id
         ) s ON TRUE
         LEFT JOIN LATERAL (
             SELECT count(DISTINCT course_id) AS started,
                    count(DISTINCT course_id) FILTER (WHERE status = 'completed') AS completed
             FROM course_sessions WHERE user_id = u.id
         ) c ON TRUE
         LEFT JOIN learner_projects p ON p.user_id = u.id
         WHERE {FILTER}
         ORDER BY u.created_at DESC, u.id DESC
         LIMIT $3 OFFSET $4"
    )))
    .bind(&pattern)
    .bind(&status)
    .bind(params.limit())
    .bind(params.offset())
    .fetch_all(&state.db)
    .await?;

    let mut summary = sqlx::query_as::<_, UserSummary>(
        "SELECT count(*) AS total,
                count(*) FILTER (WHERE email_verified_at IS NOT NULL) AS verified,
                count(*) FILTER (WHERE created_at > now() - interval '24 hours') AS last_24_hours,
                count(*) FILTER (WHERE created_at > now() - interval '7 days') AS last_7_days,
                count(*) FILTER (WHERE created_at > now() - interval '30 days') AS last_30_days
         FROM users",
    )
    .fetch_one(&state.db)
    .await?;
    summary.signups_per_day = sqlx::query_as::<_, SignupDay>(
        "SELECT (created_at AT TIME ZONE 'UTC')::date AS day, count(*) AS count
         FROM users WHERE created_at > now() - interval '30 days'
         GROUP BY day ORDER BY day",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(AdminUsersResponse {
        summary,
        users: Page::new(rows, params, total.max(0) as u64),
    }))
}
