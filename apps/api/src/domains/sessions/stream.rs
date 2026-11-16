use std::convert::Infallible;
use std::time::Duration;

use axum::extract::{Path, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt};
use uuid::Uuid;

use super::repository;
use crate::error::{ApiError, ApiResult};
use crate::http::auth::CurrentUser;
use crate::state::AppState;

/// Pushes a notification whenever the session changes, including changes made by
/// the learner's agent over MCP. Clients refetch progress on each event.
pub async fn stream(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(session_id): Path<Uuid>,
) -> ApiResult<Sse<impl Stream<Item = Result<Event, Infallible>>>> {
    repository::find_for_user(&state.db, session_id, user.id)
        .await?
        .ok_or(ApiError::NotFound("session"))?;
    let user_id = user.id;
    let updates =
        BroadcastStream::new(state.hub.subscribe()).filter_map(move |update| match update {
            Ok(update) if update.session_id == session_id && update.user_id == user_id => {
                Event::default()
                    .event(update.kind)
                    .json_data(&update)
                    .ok()
                    .map(Ok)
            }
            Ok(_) => None,
            // Lagged receivers missed updates; ask the client to refetch.
            Err(_) => Some(Ok(Event::default().event("resync").data("{}"))),
        });
    // An immediate first event makes buffering proxies (including Vite's dev proxy)
    // flush the response headers instead of waiting for the first real update.
    let ready = tokio_stream::once(Ok(Event::default().event("ready").data("{}")));
    Ok(Sse::new(ready.chain(updates))
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(20))))
}
