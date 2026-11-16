use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::outcome::ValidationOutcome;
use crate::step_type::StepTypeSpec;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    ConfigValidation,
    SubmissionValidation,
    EventHandling,
    /// Strips answer keys or secrets from configs before they reach learners.
    ConfigRedaction,
    /// Performs outbound network requests during validation.
    OutboundNetwork,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExtensionManifest {
    /// Stable identifier referenced from course definitions. Never rename.
    pub id: &'static str,
    pub name: &'static str,
    pub version: &'static str,
    pub description: &'static str,
    pub capabilities: Vec<Capability>,
    /// Step types this extension contributes to the engine.
    pub step_types: Vec<StepTypeSpec>,
    /// Event kinds this extension reacts to.
    pub event_kinds: Vec<&'static str>,
}

/// Everything an extension may know about the step it is evaluating.
#[derive(Debug, Clone, Copy)]
pub struct StepContext<'a> {
    pub user_id: &'a str,
    pub session_id: &'a str,
    pub course_id: &'a str,
    pub step_id: &'a str,
    pub step_type: &'a str,
    /// Learner-visible step configuration.
    pub config: &'a Value,
    /// Private validation rules; never exposed to learners or agents.
    pub validation: &'a Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubmissionSource {
    Web,
    Agent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Submission {
    pub source: SubmissionSource,
    #[serde(default)]
    pub payload: Value,
}

/// Something that happened in the platform that extensions may react to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionEvent {
    /// Namespaced kind, e.g. `mcp.tool_called`.
    pub kind: String,
    #[serde(default)]
    pub payload: Value,
}

impl ExtensionEvent {
    pub fn new(kind: impl Into<String>, payload: Value) -> Self {
        Self {
            kind: kind.into(),
            payload,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ExtensionError {
    #[error("invalid submission: {0}")]
    InvalidSubmission(String),
    #[error("not supported: {0}")]
    Unsupported(String),
    #[error("validation could not run: {0}")]
    Unavailable(String),
}

#[async_trait]
pub trait Extension: Send + Sync + 'static {
    fn manifest(&self) -> &ExtensionManifest;

    /// Called when a course is loaded. Return human-readable problems.
    fn validate_config(
        &self,
        step_type: &str,
        config: &Value,
        validation: &Value,
    ) -> Result<(), Vec<String>>;

    /// Config as exposed to learners and agents.
    fn public_config(&self, _step_type: &str, config: &Value) -> Value {
        config.clone()
    }

    /// Work the learner's browser performs before submitting, such as requests to send to
    /// the learner's own service. Published with the step, so it must never contain the
    /// private expectations from `validation`.
    fn client_task(&self, _step_type: &str, _config: &Value, _validation: &Value) -> Option<Value> {
        None
    }

    /// JSON Schema describing the evidence an agent should submit, if any.
    fn submission_schema(&self, _step_type: &str, _config: &Value) -> Option<Value> {
        None
    }

    async fn validate_submission(
        &self,
        ctx: &StepContext<'_>,
        _submission: &Submission,
    ) -> Result<ValidationOutcome, ExtensionError> {
        Err(ExtensionError::Unsupported(format!(
            "`{}` does not accept submissions for `{}` steps",
            self.manifest().id,
            ctx.step_type
        )))
    }

    fn handle_event(
        &self,
        _ctx: &StepContext<'_>,
        _event: &ExtensionEvent,
    ) -> Option<ValidationOutcome> {
        None
    }
}

/// Deserialize a step config section into a typed struct, reporting errors as config issues.
/// `Null` is treated as an empty object so optional sections can be omitted.
pub fn parse_config<T: DeserializeOwned>(section: &str, value: &Value) -> Result<T, Vec<String>> {
    let value = if value.is_null() {
        Value::Object(Default::default())
    } else {
        value.clone()
    };
    serde_json::from_value(value).map_err(|error| vec![format!("{section}: {error}")])
}
