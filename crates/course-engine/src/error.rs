use std::fmt;

use lp_core::{ExtensionError, StepStatus};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CourseIssue {
    /// Location inside the definition, e.g. `sections[1].steps[2].config`.
    pub path: String,
    pub message: String,
}

impl CourseIssue {
    pub fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for CourseIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("course `{course_id}` is invalid ({} issue(s))", issues.len())]
pub struct CourseValidationError {
    pub course_id: String,
    pub issues: Vec<CourseIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EngineError {
    #[error("step `{0}` does not exist in this course")]
    UnknownStep(String),
    #[error("step `{step_id}` is {status} and cannot be acted on")]
    StepNotActionable { step_id: String, status: StepStatus },
    #[error("step `{step_id}` does not accept {action}")]
    UnsupportedAction {
        step_id: String,
        action: &'static str,
    },
    #[error("extension `{0}` is not registered")]
    MissingExtension(String),
    #[error(transparent)]
    Extension(#[from] ExtensionError),
}
