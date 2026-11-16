//! Serde model of course files. This is the authoring format; it is stored verbatim
//! per course version so historical sessions can always be recompiled.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CourseDefinition {
    pub id: String,
    pub version: String,
    pub title: String,
    pub summary: String,
    pub description: String,
    pub category: String,
    pub difficulty: Difficulty,
    pub estimated_minutes: u32,
    /// Position within its difficulty level in the catalog; lower comes first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<u32>,
    /// Coaching notes added to every step's help prompt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub objectives: Vec<String>,
    #[serde(default)]
    pub prerequisites: Vec<PrerequisiteDefinition>,
    /// Extensions the course relies on. Validated against actual step usage.
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub shared_sections: BTreeMap<String, SharedSectionDefinition>,
    pub sections: Vec<SectionDefinition>,
    #[serde(default)]
    pub completion: CompletionDefinition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Difficulty {
    Beginner,
    Intermediate,
    Advanced,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrerequisiteDefinition {
    pub title: String,
    /// Set when the prerequisite is another course on the platform.
    #[serde(default)]
    pub course: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SectionDefinition {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    pub steps: Vec<StepDefinition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedSectionDefinition {
    #[serde(default)]
    pub description: Option<String>,
    pub steps: Vec<StepDefinition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepDefinition {
    pub id: String,
    #[serde(rename = "type")]
    pub step_type: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Markdown shown to the learner.
    #[serde(default)]
    pub body: Option<String>,
    /// Required for generic step types; defaults to the extension that provides the type.
    #[serde(default)]
    pub extension: Option<String>,
    #[serde(default)]
    pub config: Value,
    /// Private rules evaluated by the extension. Never sent to clients.
    #[serde(default)]
    pub validation: Value,
    /// Omitted: depends on the previous step. Empty list: available immediately.
    #[serde(default)]
    pub depends_on: Option<Vec<String>>,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub completion: Option<CompletionMode>,
    #[serde(default)]
    pub estimated_minutes: Option<u32>,
    /// Coaching notes for the help prompt: how an agent should guide without solving.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletionMode {
    /// Use the completion strategy of the step type.
    Auto,
    /// The learner may mark the step done themselves.
    Manual,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompletionDefinition {
    /// Steps that must be resolved for the course to complete. Defaults to every
    /// non-optional step.
    #[serde(default)]
    pub required_steps: Option<Vec<String>>,
}
