use std::sync::Arc;
use std::time::Duration;

use axum::extract::FromRef;
use lp_course_engine::{CourseEngine, ThemeCatalog};
use sqlx::PgPool;

use crate::config::AppConfig;
use crate::domains::courses::catalog::CourseCatalog;
use crate::http::rate_limit::RateLimiter;
use crate::mail::Mailer;
use crate::realtime::SessionHub;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub db: PgPool,
    pub engine: Arc<CourseEngine>,
    pub catalog: Arc<CourseCatalog>,
    pub themes: Arc<ThemeCatalog>,
    pub hub: SessionHub,
    pub auth_limiter: RateLimiter,
    pub mcp_limiter: RateLimiter,
    /// Emails per recipient or account, so nobody can flood an inbox.
    pub mail_limiter: RateLimiter,
    pub mailer: Arc<dyn Mailer>,
}

impl AppState {
    pub fn new(
        config: AppConfig,
        db: PgPool,
        engine: Arc<CourseEngine>,
        catalog: CourseCatalog,
        themes: ThemeCatalog,
        mailer: Arc<dyn Mailer>,
    ) -> Self {
        Self {
            config: Arc::new(config),
            db,
            engine,
            catalog: Arc::new(catalog),
            themes: Arc::new(themes),
            hub: SessionHub::new(1024),
            auth_limiter: RateLimiter::new(20, Duration::from_secs(60)),
            mcp_limiter: RateLimiter::new(240, Duration::from_secs(60)),
            mail_limiter: RateLimiter::new(5, Duration::from_secs(60 * 60)),
            mailer,
        }
    }
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}
