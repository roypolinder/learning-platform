//! Step types owned by the engine itself. They need no integration, only the
//! learner's acknowledgement or state the engine already holds.

use lp_core::{CompletionStrategy, Diagram, SlotRule, StepCategory, StepTypeSpec, parse_config};
use serde::Deserialize;
use serde_json::Value;

use crate::definition::StepDefinition;

pub const REUSABLE_SECTION: &str = "reusable_section";
pub const CONDITIONAL: &str = "conditional";
pub const PREREQUISITE: &str = "prerequisite";
pub const PROJECT_SETUP: &str = "project_setup";

const SPECS: &[StepTypeSpec] = &[
    StepTypeSpec::new(
        "explanation",
        StepCategory::Content,
        CompletionStrategy::Acknowledge,
        "Markdown explanation the learner reads.",
    ),
    StepTypeSpec::new(
        "diagram",
        StepCategory::Content,
        CompletionStrategy::Acknowledge,
        "Boxes-and-arrows diagram; learners can select a node for its explanation.",
    ),
    StepTypeSpec::new(
        "video",
        StepCategory::Content,
        CompletionStrategy::Acknowledge,
        "Externally hosted video with optional transcript.",
    ),
    StepTypeSpec::new(
        "code_example",
        StepCategory::Content,
        CompletionStrategy::Acknowledge,
        "One or more annotated code snippets.",
    ),
    StepTypeSpec::new(
        "external_task",
        StepCategory::Practical,
        CompletionStrategy::Submission,
        "Work done outside the platform, verified by an extension or self-reported.",
    ),
    StepTypeSpec::new(
        "extension_validation",
        StepCategory::Practical,
        CompletionStrategy::Submission,
        "Evidence validated by an explicitly referenced extension.",
    ),
    StepTypeSpec::new(
        CONDITIONAL,
        StepCategory::Workflow,
        CompletionStrategy::Derived,
        "Shown only when a condition on an earlier step holds; skipped otherwise.",
    ),
    StepTypeSpec::new(
        PREREQUISITE,
        StepCategory::Workflow,
        CompletionStrategy::Derived,
        "Completes once the learner has finished the referenced courses.",
    ),
    StepTypeSpec::new(
        PROJECT_SETUP,
        StepCategory::Workflow,
        CompletionStrategy::Derived,
        "The learner chooses the website they build throughout the track. Completes once chosen.",
    ),
    StepTypeSpec::new(
        "manual_completion",
        StepCategory::Workflow,
        CompletionStrategy::Acknowledge,
        "Learner marks the step done.",
    ),
    StepTypeSpec::new(
        "automatic_completion",
        StepCategory::Workflow,
        CompletionStrategy::Automatic,
        "Completes as soon as it unlocks.",
    ),
];

pub fn specs() -> &'static [StepTypeSpec] {
    SPECS
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Condition {
    pub step: String,
    #[serde(default)]
    pub status: Option<lp_core::StepStatus>,
    #[serde(default)]
    pub score_below: Option<f64>,
    #[serde(default)]
    pub score_at_least: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConditionalConfig {
    pub when: Condition,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrerequisiteConfig {
    pub courses: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReusableSectionConfig {
    pub section: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct VideoConfig {
    url: String,
    #[serde(default)]
    duration_seconds: Option<u32>,
    #[serde(default)]
    transcript: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CodeExampleConfig {
    snippets: Vec<Snippet>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct Snippet {
    language: String,
    code: String,
    #[serde(default)]
    filename: Option<String>,
    #[serde(default)]
    caption: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct InstructionsConfig {
    #[serde(default)]
    instructions: Option<String>,
}

pub fn validate(step: &StepDefinition) -> Vec<String> {
    let result = match step.step_type.as_str() {
        "explanation" => require_body(step),
        "diagram" => parse_config::<Diagram>("config", &step.config).and_then(|diagram| {
            let issues = diagram.check(SlotRule::Forbidden);
            if issues.is_empty() {
                Ok(())
            } else {
                Err(issues)
            }
        }),
        "video" => parse_config::<VideoConfig>("config", &step.config).and_then(|config| {
            if config.url.starts_with("https://") {
                Ok(())
            } else {
                Err(vec!["config.url must use https".into()])
            }
        }),
        "code_example" => {
            parse_config::<CodeExampleConfig>("config", &step.config).and_then(|config| {
                if config.snippets.is_empty() {
                    Err(vec!["config.snippets must not be empty".into()])
                } else {
                    Ok(())
                }
            })
        }
        CONDITIONAL => parse_config::<ConditionalConfig>("config", &step.config).and_then(|c| {
            let when = c.when;
            let criteria = [
                when.status.is_some(),
                when.score_below.is_some(),
                when.score_at_least.is_some(),
            ];
            if criteria.iter().filter(|set| **set).count() == 1 {
                Ok(())
            } else {
                Err(vec![
                    "config.when needs exactly one of status, score_below, score_at_least".into(),
                ])
            }
        }),
        PREREQUISITE => parse_config::<PrerequisiteConfig>("config", &step.config).and_then(|c| {
            if c.courses.is_empty() {
                Err(vec!["config.courses must not be empty".into()])
            } else {
                Ok(())
            }
        }),
        "manual_completion" | "automatic_completion" | PROJECT_SETUP => {
            parse_config::<InstructionsConfig>("config", &step.config).map(|_| ())
        }
        _ => Ok(()),
    };
    result.err().unwrap_or_default()
}

fn require_body(step: &StepDefinition) -> Result<(), Vec<String>> {
    match &step.body {
        Some(body) if !body.trim().is_empty() => Ok(()),
        _ => Err(vec!["body is required".into()]),
    }
}

pub fn condition_of(config: &Value) -> Option<Condition> {
    parse_config::<ConditionalConfig>("config", config)
        .ok()
        .map(|config| config.when)
}

pub fn prerequisite_courses(config: &Value) -> Vec<String> {
    parse_config::<PrerequisiteConfig>("config", config)
        .map(|config| config.courses)
        .unwrap_or_default()
}
