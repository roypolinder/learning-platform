use axum::Json;
use axum::extract::State;

use super::service::{self, ProgressOverview};
use crate::error::ApiResult;
use crate::http::auth::CurrentUser;
use crate::state::AppState;

pub async fn overview(
    State(state): State<AppState>,
    user: CurrentUser,
) -> ApiResult<Json<ProgressOverview>> {
    Ok(Json(service::overview(&state, user.id).await?))
}
