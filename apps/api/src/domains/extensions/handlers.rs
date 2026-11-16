use axum::Json;
use axum::extract::State;
use lp_core::{CompletionStrategy, ExtensionManifest, StepCategory};
use serde::Serialize;

use super::repository;
use crate::error::ApiResult;
use crate::state::AppState;

#[derive(Serialize)]
pub struct ExtensionDto {
    #[serde(flatten)]
    pub manifest: ExtensionManifest,
    pub enabled: bool,
}

#[derive(Serialize)]
pub struct StepTypeDto {
    pub name: &'static str,
    pub category: StepCategory,
    pub completion: CompletionStrategy,
    pub description: &'static str,
    pub provider: Option<&'static str>,
}

pub async fn list(State(state): State<AppState>) -> ApiResult<Json<Vec<ExtensionDto>>> {
    let enabled = repository::enabled_flags(&state.db).await?;
    Ok(Json(
        state
            .engine
            .extensions()
            .manifests()
            .map(|manifest| ExtensionDto {
                enabled: enabled.get(manifest.id).copied().unwrap_or(true),
                manifest: manifest.clone(),
            })
            .collect(),
    ))
}

pub async fn step_types(State(state): State<AppState>) -> Json<Vec<StepTypeDto>> {
    Json(
        state
            .engine
            .step_types()
            .map(|entry| StepTypeDto {
                name: entry.spec.name,
                category: entry.spec.category,
                completion: entry.spec.completion,
                description: entry.spec.description,
                provider: entry.provider,
            })
            .collect(),
    )
}
