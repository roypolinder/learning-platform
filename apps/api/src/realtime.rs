//! In-process fan-out of session updates to SSE subscribers. A multi-instance
//! deployment would swap this for PostgreSQL LISTEN/NOTIFY behind the same API.

use serde::Serialize;
use tokio::sync::broadcast;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct SessionUpdate {
    pub session_id: Uuid,
    #[serde(skip)]
    pub user_id: Uuid,
    pub kind: &'static str,
    pub step_ids: Vec<String>,
    pub course_completed: bool,
}

#[derive(Clone)]
pub struct SessionHub {
    sender: broadcast::Sender<SessionUpdate>,
}

impl SessionHub {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn publish(&self, update: SessionUpdate) {
        // No subscribers is the normal case when nobody has the page open.
        let _ = self.sender.send(update);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<SessionUpdate> {
        self.sender.subscribe()
    }
}
