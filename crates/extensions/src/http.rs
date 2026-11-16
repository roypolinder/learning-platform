//! Checks a single endpoint of a service the learner runs. The learner's browser sends the
//! request (see [`crate::client_run`]), so `localhost` works and the platform makes no
//! outbound requests.

use std::collections::BTreeMap;

use async_trait::async_trait;
use lp_core::{
    Capability, CheckResult, CompletionStrategy, Extension, ExtensionError, ExtensionManifest,
    StepCategory, StepContext, StepTypeSpec, Submission, ValidationOutcome, parse_config,
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::client_run::{HttpRun, describe_error, require_browser, unreachable_outcome};
use crate::support::{collect_issues, parse_submission, unsupported_step_type};

pub const ID: &str = "http";
const STEP_TYPE: &str = "http_endpoint_check";
const CASE_ID: &str = "endpoint";

pub struct HttpExtension {
    manifest: ExtensionManifest,
}

impl Default for HttpExtension {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpExtension {
    pub fn new() -> Self {
        Self {
            manifest: ExtensionManifest {
                id: ID,
                name: "HTTP",
                version: "2.0.0",
                description: "Verifies that an endpoint of the learner's service responds as expected.",
                capabilities: vec![
                    Capability::ConfigValidation,
                    Capability::SubmissionValidation,
                ],
                step_types: vec![StepTypeSpec::new(
                    STEP_TYPE,
                    StepCategory::Practical,
                    CompletionStrategy::Submission,
                    "The learner's browser requests an endpoint; the platform checks status, body and latency.",
                )],
                event_kinds: Vec::new(),
            },
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct EndpointConfig {
    #[serde(default)]
    #[allow(dead_code)]
    instructions: Option<String>,
    /// Appended to the submitted base URL; without it the URL is requested as given.
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    method: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EndpointRules {
    #[serde(default = "ok_status")]
    expect_status: u16,
    /// JSON pointer -> expected value.
    #[serde(default)]
    expect_json: BTreeMap<String, Value>,
    #[serde(default)]
    body_contains: Vec<String>,
    #[serde(default)]
    max_latency_ms: Option<u64>,
}

fn ok_status() -> u16 {
    200
}

impl EndpointConfig {
    fn method(&self) -> Result<&'static str, String> {
        match self
            .method
            .as_deref()
            .unwrap_or("GET")
            .to_ascii_uppercase()
            .as_str()
        {
            "GET" => Ok("GET"),
            "HEAD" => Ok("HEAD"),
            other => Err(format!(
                "config.method `{other}` is not allowed; use GET or HEAD"
            )),
        }
    }
}

#[async_trait]
impl Extension for HttpExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    fn validate_config(
        &self,
        step_type: &str,
        config: &Value,
        validation: &Value,
    ) -> Result<(), Vec<String>> {
        if step_type != STEP_TYPE {
            return Err(unsupported_step_type(ID, step_type));
        }
        collect_issues([
            parse_config::<EndpointConfig>("config", config).and_then(|config| {
                let mut issues: Vec<String> = config.method().err().into_iter().collect();
                if config
                    .path
                    .as_deref()
                    .is_some_and(|path| !path.starts_with('/'))
                {
                    issues.push("config.path must start with `/`".into());
                }
                if issues.is_empty() {
                    Ok(())
                } else {
                    Err(issues)
                }
            }),
            parse_config::<EndpointRules>("validation", validation).and_then(|rules| {
                let bad: Vec<String> = rules
                    .expect_json
                    .keys()
                    .filter(|pointer| !pointer.starts_with('/'))
                    .map(|pointer| {
                        format!("validation.expect_json key `{pointer}` must be a JSON pointer")
                    })
                    .collect();
                if bad.is_empty() { Ok(()) } else { Err(bad) }
            }),
        ])
    }

    fn client_task(&self, _step_type: &str, config: &Value, _validation: &Value) -> Option<Value> {
        let config: EndpointConfig = parse_config("config", config).ok()?;
        Some(json!({
            "kind": "http",
            "timeout_ms": 5000,
            "parallel": false,
            "cases": [{
                "id": CASE_ID,
                "label": "Endpoint responds",
                "request": { "method": config.method().ok()?, "path": config.path.unwrap_or_default() }
            }]
        }))
    }

    fn submission_schema(&self, _step_type: &str, _config: &Value) -> Option<Value> {
        Some(json!({
            "type": "object",
            "description": "Produced by the web app after the learner's browser requested the endpoint.",
            "required": ["url"],
            "properties": { "url": { "type": "string", "format": "uri" } }
        }))
    }

    async fn validate_submission(
        &self,
        ctx: &StepContext<'_>,
        submission: &Submission,
    ) -> Result<ValidationOutcome, ExtensionError> {
        require_browser(submission)?;
        let rules: EndpointRules = parse_config("validation", ctx.validation)
            .map_err(|issues| ExtensionError::Unavailable(issues.join("; ")))?;
        let run = parse_submission::<HttpRun>(&submission.payload)?.checked()?;
        if run.unreachable() {
            return Ok(unreachable_outcome(&run.url));
        }
        let result = run
            .result(CASE_ID)
            .ok_or_else(|| ExtensionError::InvalidSubmission("missing endpoint result".into()))?;
        if let Some(error) = &result.error {
            return Ok(
                ValidationOutcome::failed(format!("Request to {} failed", run.url)).with_checks(
                    vec![
                        CheckResult::new("reachable", "Endpoint is reachable", false)
                            .with_detail(describe_error(error)),
                    ],
                ),
            );
        }
        let status = result.status.unwrap_or_default();
        let mut checks = vec![
            CheckResult::new("reachable", "Endpoint is reachable", true).with_detail(
                result
                    .latency_ms
                    .map(|ms| format!("{ms} ms"))
                    .unwrap_or_default(),
            ),
            CheckResult::new(
                "status",
                format!("Responds with HTTP {}", rules.expect_status),
                status == rules.expect_status,
            )
            .with_detail(format!("got {status}")),
        ];
        if let Some(limit) = rules.max_latency_ms {
            checks.push(CheckResult::new(
                "latency",
                format!("Responds within {limit} ms"),
                result.latency_ms.is_some_and(|ms| ms <= limit),
            ));
        }
        let text = result.body.as_deref().unwrap_or_default();
        for (index, needle) in rules.body_contains.iter().enumerate() {
            checks.push(CheckResult::new(
                format!("body-{index}"),
                format!("Body contains `{needle}`"),
                text.contains(needle.as_str()),
            ));
        }
        if !rules.expect_json.is_empty() {
            let parsed = serde_json::from_str::<Value>(text).ok();
            checks.push(CheckResult::new(
                "json",
                "Body is valid JSON",
                parsed.is_some(),
            ));
            for (pointer, expected) in &rules.expect_json {
                let actual = parsed.as_ref().and_then(|json| json.pointer(pointer));
                let check = CheckResult::new(
                    format!("json:{pointer}"),
                    format!("`{pointer}` equals {expected}"),
                    actual == Some(expected),
                );
                checks.push(match actual {
                    Some(actual) if actual != expected => {
                        check.with_detail(format!("got {actual}"))
                    }
                    None => check.with_detail("missing"),
                    _ => check,
                });
            }
        }
        Ok(ValidationOutcome::from_checks(
            checks,
            "Endpoint responded as expected",
            "Endpoint did not respond as expected",
        ))
    }
}

#[cfg(test)]
mod tests {
    use lp_core::{OutcomeStatus, SubmissionSource};

    use super::*;

    fn ctx<'a>(config: &'a Value, validation: &'a Value) -> StepContext<'a> {
        StepContext {
            user_id: "u",
            session_id: "s",
            course_id: "c",
            step_id: "health",
            step_type: STEP_TYPE,
            config,
            validation,
        }
    }

    fn submit(result: Value) -> Submission {
        Submission {
            source: SubmissionSource::Web,
            payload: json!({ "url": "http://localhost:3000", "results": [result] }),
        }
    }

    #[tokio::test]
    async fn checks_status_and_json_body() {
        let extension = HttpExtension::new();
        let config = json!({ "path": "/health" });
        let healthy = json!({ "id": "endpoint", "status": 200, "latency_ms": 4, "body": "{\"status\":\"ok\"}" });

        let ok = json!({ "expect_json": { "/status": "ok" } });
        let outcome = extension
            .validate_submission(&ctx(&config, &ok), &submit(healthy.clone()))
            .await
            .unwrap();
        assert!(outcome.is_passed(), "{outcome:?}");

        let wrong = json!({ "expect_json": { "/status": "degraded" } });
        let outcome = extension
            .validate_submission(&ctx(&config, &wrong), &submit(healthy))
            .await
            .unwrap();
        assert_eq!(outcome.status, OutcomeStatus::Failed);
    }

    #[test]
    fn publishes_the_request_but_not_the_rules() {
        let task = HttpExtension::new()
            .client_task(
                STEP_TYPE,
                &json!({ "path": "/health" }),
                &json!({ "expect_status": 200 }),
            )
            .unwrap();
        assert_eq!(task["cases"][0]["request"]["path"], "/health");
        assert!(!task.to_string().contains("expect_status"));
    }
}
