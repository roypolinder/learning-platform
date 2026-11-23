//! Stores warnings, errors and audit events in `system_logs`, so super admins can look them
//! up in the app instead of on the server. Everything still goes to stdout (the journal) too.
//!
//! Log an audit event with `tracing::info!(target: AUDIT, category = "…", …)`. Never put
//! secrets (tokens, passwords, links with tokens) in such events.

use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde_json::{Map, Value};
use sqlx::PgPool;
use tokio::sync::mpsc;
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::{Context, Layer};
use uuid::Uuid;

/// Target for INFO events that should be stored, e.g. "email sent".
pub const AUDIT: &str = "audit";
/// Stored rows are kept this long (see the privacy page).
pub const RETENTION_DAYS: i32 = 30;
const BUFFER: usize = 10_000;

#[derive(Debug, Clone)]
pub struct LogRecord {
    pub created_at: DateTime<Utc>,
    pub level: &'static str,
    pub target: String,
    pub category: Option<String>,
    pub message: String,
    pub fields: Map<String, Value>,
}

static RECEIVER: OnceLock<Mutex<Option<mpsc::Receiver<LogRecord>>>> = OnceLock::new();

/// The layer to install next to the console output. Records wait in a bounded buffer until
/// [`start_writer`] has a database; when the buffer is full, new records are dropped rather
/// than slowing requests down.
pub fn layer() -> StoredLogLayer {
    let (sender, receiver) = mpsc::channel(BUFFER);
    let _ = RECEIVER.set(Mutex::new(Some(receiver)));
    StoredLogLayer { sender }
}

pub struct StoredLogLayer {
    sender: mpsc::Sender<LogRecord>,
}

impl<S: Subscriber> Layer<S> for StoredLogLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let level = match *metadata.level() {
            Level::ERROR => "error",
            Level::WARN => "warn",
            Level::INFO if metadata.target() == AUDIT => "info",
            _ => return,
        };
        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);
        let category = visitor
            .fields
            .remove("category")
            .and_then(|value| value.as_str().map(str::to_owned));
        let _ = self.sender.try_send(LogRecord {
            created_at: Utc::now(),
            level,
            target: metadata.target().to_owned(),
            category,
            message: visitor.message.unwrap_or_default(),
            fields: visitor.fields,
        });
    }
}

#[derive(Default)]
struct FieldVisitor {
    message: Option<String>,
    fields: Map<String, Value>,
}

impl FieldVisitor {
    fn put(&mut self, field: &Field, value: Value) {
        if field.name() == "message" {
            self.message = Some(match value {
                Value::String(text) => text,
                other => other.to_string(),
            });
        } else {
            self.fields.insert(field.name().to_owned(), value);
        }
    }
}

impl Visit for FieldVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.put(field, Value::String(value.to_owned()));
    }
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.put(field, value.into());
    }
    fn record_u64(&mut self, field: &Field, value: u64) {
        self.put(field, value.into());
    }
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.put(field, value.into());
    }
    fn record_f64(&mut self, field: &Field, value: f64) {
        self.put(field, value.into());
    }
    fn record_error(&mut self, field: &Field, value: &(dyn std::error::Error + 'static)) {
        self.put(field, Value::String(value.to_string()));
    }
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.put(field, Value::String(format!("{value:?}")));
    }
}

/// Starts writing buffered records to the database and pruning old ones. Does nothing when
/// the layer is not installed (tests) or the writer already runs.
pub fn start_writer(pool: PgPool) {
    let Some(mut receiver) = RECEIVER.get().and_then(|slot| slot.lock().ok()?.take()) else {
        return;
    };
    let writer_pool = pool.clone();
    tokio::spawn(async move {
        while let Some(record) = receiver.recv().await {
            // Failures go to stderr only: logging them through tracing would loop.
            if let Err(error) = insert(&writer_pool, &record).await {
                eprintln!("could not store log record: {error}");
            }
        }
    });
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60 * 60));
        loop {
            interval.tick().await;
            if let Err(error) = prune(&pool).await {
                eprintln!("could not prune system logs: {error}");
            }
        }
    });
}

pub async fn insert(pool: &PgPool, record: &LogRecord) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO system_logs (id, created_at, level, target, category, message, fields)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(Uuid::now_v7())
    .bind(record.created_at)
    .bind(record.level)
    .bind(&record.target)
    .bind(&record.category)
    .bind(&record.message)
    .bind(Value::Object(record.fields.clone()))
    .execute(pool)
    .await?;
    Ok(())
}

async fn prune(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM system_logs WHERE created_at < now() - make_interval(days => $1)")
        .bind(RETENTION_DAYS)
        .execute(pool)
        .await?;
    Ok(())
}
