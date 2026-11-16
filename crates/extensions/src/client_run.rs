//! Results of requests the learner's browser sent to the service they built. The browser can
//! reach `localhost` and private networks, so learners never expose their app, and the
//! platform itself makes no outbound requests on a learner's behalf.
//!
//! The browser reports raw responses; whether they are right is decided here, against
//! expectations that never leave the server.

use lp_core::{CheckResult, ExtensionError, Submission, SubmissionSource, ValidationOutcome};
use serde::Deserialize;
use url::Url;

const MAX_RESULTS: usize = 64;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HttpRun {
    pub url: String,
    /// Random value the browser substituted for `{{nonce}}` in this run.
    #[serde(default)]
    pub nonce: String,
    #[serde(default)]
    pub results: Vec<CaseRun>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseRun {
    pub id: String,
    #[serde(default)]
    pub status: Option<u16>,
    #[serde(default)]
    pub latency_ms: Option<u64>,
    #[serde(default)]
    pub body: Option<String>,
    /// `network`, `timeout` or a short message when no response was received.
    #[serde(default)]
    pub error: Option<String>,
}

impl HttpRun {
    pub fn result(&self, id: &str) -> Option<&CaseRun> {
        self.results.iter().find(|run| run.id == id)
    }

    pub fn checked(self) -> Result<Self, ExtensionError> {
        base_url(&self.url)?;
        if self.results.len() > MAX_RESULTS {
            return Err(ExtensionError::InvalidSubmission("too many results".into()));
        }
        Ok(self)
    }

    /// Every request failed without a response: the service is down or blocks the browser.
    pub fn unreachable(&self) -> bool {
        !self.results.is_empty()
            && self
                .results
                .iter()
                .all(|run| run.status.is_none() && run.error.as_deref() == Some("network"))
    }
}

/// Learners run these checks from the web app. Agents can't submit them, so results always
/// come from a real request made by the learner's browser.
pub fn require_browser(submission: &Submission) -> Result<(), ExtensionError> {
    match submission.source {
        SubmissionSource::Web => Ok(()),
        SubmissionSource::Agent => Err(ExtensionError::InvalidSubmission(
            "the learner runs this check from the step page in the web app".into(),
        )),
    }
}

pub fn base_url(raw: &str) -> Result<Url, ExtensionError> {
    let url = Url::parse(raw.trim())
        .map_err(|error| ExtensionError::InvalidSubmission(format!("invalid URL: {error}")))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(ExtensionError::InvalidSubmission(
            "only http and https URLs are allowed".into(),
        ));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(ExtensionError::InvalidSubmission(
            "URLs must not contain credentials".into(),
        ));
    }
    Ok(url)
}

pub fn check_nonce(nonce: &str) -> Result<(), ExtensionError> {
    let valid = (4..=32).contains(&nonce.len())
        && nonce
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-');
    if valid {
        Ok(())
    } else {
        Err(ExtensionError::InvalidSubmission(
            "nonce must be 4 to 32 letters, digits or dashes".into(),
        ))
    }
}

pub fn describe_error(error: &str) -> String {
    match error {
        "timeout" => "timed out".into(),
        "network" => {
            "your browser got no response: is the service running, and does it allow requests from this platform (CORS)?".into()
        }
        other => other.chars().take(160).collect(),
    }
}

pub fn unreachable_outcome(url: &str) -> ValidationOutcome {
    let reason = describe_error("network");
    ValidationOutcome::failed(format!("Could not reach {url}")).with_checks(vec![
        CheckResult::new("reachable", "Your browser can reach the service", false)
            .with_detail(reason),
    ])
}
