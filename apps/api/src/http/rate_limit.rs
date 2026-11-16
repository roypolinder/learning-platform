//! Fixed-window limiter kept in memory. Adequate for a single instance; a
//! horizontally scaled deployment would move this to the reverse proxy.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::error::ApiError;

const MAX_TRACKED_KEYS: usize = 10_000;

#[derive(Clone)]
pub struct RateLimiter {
    inner: Arc<Mutex<HashMap<String, (Instant, u32)>>>,
    limit: u32,
    window: Duration,
}

impl RateLimiter {
    pub fn new(limit: u32, window: Duration) -> Self {
        Self {
            inner: Arc::default(),
            limit,
            window,
        }
    }

    pub fn check(&self, key: &str) -> Result<(), ApiError> {
        let now = Instant::now();
        let mut windows = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if windows.len() > MAX_TRACKED_KEYS {
            let window = self.window;
            windows.retain(|_, (started, _)| now.duration_since(*started) < window);
        }
        let entry = windows.entry(key.to_owned()).or_insert((now, 0));
        if now.duration_since(entry.0) >= self.window {
            *entry = (now, 0);
        }
        entry.1 += 1;
        if entry.1 > self.limit {
            let retry_after = self.window.saturating_sub(now.duration_since(entry.0));
            return Err(ApiError::RateLimited {
                retry_after_seconds: retry_after.as_secs().max(1),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_after_limit_per_key() {
        let limiter = RateLimiter::new(2, Duration::from_secs(60));
        assert!(limiter.check("a").is_ok());
        assert!(limiter.check("a").is_ok());
        assert!(matches!(
            limiter.check("a"),
            Err(ApiError::RateLimited { .. })
        ));
        assert!(limiter.check("b").is_ok());
    }
}
