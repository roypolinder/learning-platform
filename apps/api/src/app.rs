use std::sync::Arc;
use std::time::Duration;

use axum::extract::State;
use axum::http::{HeaderName, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router, middleware};
use lp_course_engine::{CourseEngine, ExtensionRegistry};
use lp_extensions::{ExtensionOptions, builtin_extensions};
use serde_json::json;
use tokio_util::sync::CancellationToken;
use tower_http::compression::CompressionLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

use crate::config::AppConfig;
use crate::db;
use crate::domains::{
    auth, authoring, courses, extensions, integrations, oauth, progress, sessions, traffic, users,
};
use crate::error::ApiError;
use crate::http::csrf;
use crate::mcp;
use crate::state::AppState;

#[derive(Debug, thiserror::Error)]
pub enum StartupError {
    #[error("database connection failed: {0}")]
    Database(#[from] sqlx::Error),
    #[error("migrations failed: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("extension registry is invalid: {0}")]
    Registry(#[from] lp_course_engine::RegistryError),
    #[error(transparent)]
    Courses(#[from] courses::sync::SyncError),
}

pub fn build_engine(config: &AppConfig) -> Result<CourseEngine, StartupError> {
    let assets = config.courses_dir.join("assets");
    let options = ExtensionOptions {
        assets_dir: assets.is_dir().then_some(assets),
    };
    let mut registry = ExtensionRegistry::new();
    for extension in builtin_extensions(&options) {
        registry.register(extension)?;
    }
    Ok(CourseEngine::new(registry)?)
}

/// Connects, migrates, registers extensions and publishes course files.
pub async fn bootstrap(config: AppConfig) -> Result<AppState, StartupError> {
    let pool = db::connect(&config.database_url, config.database_max_connections).await?;
    bootstrap_with_pool(config, pool).await
}

pub async fn bootstrap_with_pool(
    config: AppConfig,
    pool: sqlx::PgPool,
) -> Result<AppState, StartupError> {
    db::migrate(&pool).await?;
    let promoted =
        users::repository::promote_super_admins(&pool, &config.super_admin_emails).await?;
    if promoted > 0 {
        tracing::info!(promoted, "granted super admin from SUPER_ADMIN_EMAILS");
    }
    let engine = Arc::new(build_engine(&config)?);
    extensions::repository::sync(&pool, engine.extensions().manifests()).await?;
    let catalog = courses::sync::sync_courses(
        &pool,
        &engine,
        &config.courses_dir,
        config.mutable_course_versions,
    )
    .await?;
    Ok(AppState::new(config, pool, engine, catalog))
}

pub fn router(state: AppState, shutdown: CancellationToken) -> Router {
    let api = Router::new()
        .merge(auth::routes())
        .merge(users::routes())
        .merge(courses::routes())
        .merge(sessions::routes())
        .merge(progress::routes())
        .merge(extensions::routes())
        .merge(integrations::routes())
        .merge(authoring::routes())
        .merge(traffic::routes())
        .merge(oauth::routes())
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(30),
        ))
        .merge(sessions::stream_routes())
        .fallback(|| async { ApiError::NotFound("route") })
        .layer(middleware::from_fn(csrf::require_client_header));

    let mut app = Router::new()
        .route("/api/health", get(health))
        .nest("/api/v1", api)
        .merge(oauth::public_routes())
        .with_state(state.clone())
        .merge(mcp::router(state.clone(), shutdown));

    if let Some(dist) = state
        .config
        .web_dist_dir
        .as_ref()
        .filter(|dir| dir.is_dir())
    {
        let index = dist.join("index.html");
        app = app.fallback_service(
            ServeDir::new(dist)
                .precompressed_gzip()
                .fallback(ServeFile::new(index)),
        );
    }

    app.layer(RequestBodyLimitLayer::new(2 * 1024 * 1024))
        .layer(CompressionLayer::new())
        .layer(security_header("x-content-type-options", "nosniff"))
        .layer(security_header("x-frame-options", "DENY"))
        .layer(security_header(
            "referrer-policy",
            "strict-origin-when-cross-origin",
        ))
        .layer(TraceLayer::new_for_http())
}

fn security_header(name: &'static str, value: &'static str) -> SetResponseHeaderLayer<HeaderValue> {
    SetResponseHeaderLayer::if_not_present(
        HeaderName::from_static(name),
        HeaderValue::from_static(value),
    )
}

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let database = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.db)
        .await
        .is_ok();
    let status = if database {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (
        status,
        Json(json!({
            "status": if database { "ok" } else { "degraded" },
            "version": env!("CARGO_PKG_VERSION"),
            "database": if database { "ok" } else { "unreachable" },
            "courses": state.catalog.list().len(),
            "extensions": state.engine.extensions().len(),
        })),
    )
}
