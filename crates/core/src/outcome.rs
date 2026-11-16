use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeStatus {
    Passed,
    Failed,
    /// Accepted but not yet decisive, e.g. waiting for a required event.
    Pending,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckResult {
    pub id: String,
    pub label: String,
    pub passed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl CheckResult {
    pub fn new(id: impl Into<String>, label: impl Into<String>, passed: bool) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            passed,
            detail: None,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationOutcome {
    pub status: OutcomeStatus,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    #[serde(default)]
    pub checks: Vec<CheckResult>,
    /// Extension-specific, learner-safe feedback (e.g. per-question explanations).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feedback: Option<Value>,
}

impl ValidationOutcome {
    pub fn passed(summary: impl Into<String>) -> Self {
        Self::new(OutcomeStatus::Passed, summary)
    }

    pub fn failed(summary: impl Into<String>) -> Self {
        Self::new(OutcomeStatus::Failed, summary)
    }

    pub fn pending(summary: impl Into<String>) -> Self {
        Self::new(OutcomeStatus::Pending, summary)
    }

    fn new(status: OutcomeStatus, summary: impl Into<String>) -> Self {
        Self {
            status,
            summary: summary.into(),
            score: None,
            checks: Vec::new(),
            feedback: None,
        }
    }

    /// Passes only if every check passes.
    pub fn from_checks(checks: Vec<CheckResult>, passed: &str, failed: &str) -> Self {
        let failures = checks.iter().filter(|check| !check.passed).count();
        let mut outcome = if failures == 0 {
            Self::passed(passed)
        } else {
            Self::failed(format!(
                "{failed} ({failures} of {} checks failed)",
                checks.len()
            ))
        };
        outcome.checks = checks;
        outcome
    }

    pub fn with_score(mut self, score: f64) -> Self {
        self.score = Some(score);
        self
    }

    pub fn with_checks(mut self, checks: Vec<CheckResult>) -> Self {
        self.checks = checks;
        self
    }

    pub fn with_feedback(mut self, feedback: Value) -> Self {
        self.feedback = Some(feedback);
        self
    }

    pub fn is_passed(&self) -> bool {
        self.status == OutcomeStatus::Passed
    }
}
