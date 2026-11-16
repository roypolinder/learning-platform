use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StepCategory {
    Content,
    Assessment,
    Practical,
    Workflow,
}

/// How a step of a given type reaches `completed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletionStrategy {
    /// The learner confirms they have read or done it.
    Acknowledge,
    /// A submission is validated by the step's extension.
    Submission,
    /// An extension observes platform events (e.g. an MCP client connecting).
    Event,
    /// Completes as soon as it unlocks.
    Automatic,
    /// Resolved by the engine from other state (conditions, prerequisites).
    Derived,
}

#[derive(Debug, Clone, Serialize)]
pub struct StepTypeSpec {
    pub name: &'static str,
    pub category: StepCategory,
    pub completion: CompletionStrategy,
    pub description: &'static str,
}

impl StepTypeSpec {
    pub const fn new(
        name: &'static str,
        category: StepCategory,
        completion: CompletionStrategy,
        description: &'static str,
    ) -> Self {
        Self {
            name,
            category,
            completion,
            description,
        }
    }
}
