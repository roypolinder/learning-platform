//! Hidden test cases for a service or MCP server the learner built — e.g. "searching for
//! *money* ranks a finance document first" or "the agent refuses a destructive task".
//!
//! The learner's browser sends the requests (see [`crate::client_run`]), so services on
//! `localhost` work without being exposed. Only the requests are published; expectations
//! stay in the step's private `validation`, so passing requires behaviour that works.

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

use async_trait::async_trait;
use lp_core::{
    Capability, CheckResult, CompletionStrategy, Extension, ExtensionError, ExtensionManifest,
    StepCategory, StepContext, StepTypeSpec, Submission, ValidationOutcome, parse_config,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::client_run::{
    CaseRun, HttpRun, base_url, check_nonce, describe_error, require_browser, unreachable_outcome,
};
use crate::rules::{JsonRule, NONCE_TOKEN};
use crate::support::{collect_issues, parse_submission, unsupported_step_type};

pub const ID: &str = "service";
const SERVICE_CHECK: &str = "service_check";
const MCP_SERVER_CHECK: &str = "mcp_server_check";

const DEFAULT_TIMEOUT_MS: u64 = 15_000;
const MAX_TIMEOUT_MS: u64 = 120_000;
const MAX_ASSET_BYTES: u64 = 2 * 1024 * 1024;
const ASSET_PREFIX: &str = "{{asset:";

#[derive(Debug, Clone, Default)]
pub struct ServiceOptions {
    /// Directory with request fixtures such as images (`{{asset:name.png}}`).
    pub assets_dir: Option<PathBuf>,
}

pub struct ServiceExtension {
    manifest: ExtensionManifest,
    options: ServiceOptions,
}

impl ServiceExtension {
    pub fn new(options: ServiceOptions) -> Self {
        Self {
            options,
            manifest: ExtensionManifest {
                id: ID,
                name: "Service checks",
                version: "2.0.0",
                description: "Runs hidden test cases against a service or MCP server the learner built.",
                capabilities: vec![
                    Capability::ConfigValidation,
                    Capability::SubmissionValidation,
                ],
                step_types: vec![
                    StepTypeSpec::new(
                        SERVICE_CHECK,
                        StepCategory::Practical,
                        CompletionStrategy::Submission,
                        "The learner's browser sends test requests to their service; the platform checks the responses.",
                    ),
                    StepTypeSpec::new(
                        MCP_SERVER_CHECK,
                        StepCategory::Practical,
                        CompletionStrategy::Submission,
                        "The learner's browser connects to their MCP server, lists and calls its tools; the platform checks the results.",
                    ),
                ],
                event_kinds: Vec::new(),
            },
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ServiceConfig {
    /// Prefilled URL in the web UI.
    #[serde(default)]
    default_url: Option<String>,
    /// MCP endpoint path for `mcp_server_check`.
    #[serde(default)]
    path: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    #[serde(default = "default_timeout")]
    timeout_ms: u64,
    /// Lets the browser run cases concurrently. Only for cases without ordering.
    #[serde(default)]
    parallel: bool,
    /// Share of `graded` cases that must pass; ungraded cases must all pass.
    #[serde(default = "full_ratio")]
    min_pass_ratio: f64,
    cases: Vec<Case>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    id: String,
    label: String,
    request: RequestSpec,
    #[serde(default)]
    expect: Expect,
    #[serde(default)]
    graded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestSpec {
    #[serde(default = "get")]
    method: String,
    path: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    query: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    json: Option<Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Expect {
    /// Defaults to any 2xx.
    #[serde(default)]
    status: Option<StatusExpectation>,
    #[serde(default)]
    json: Vec<JsonRule>,
    #[serde(default)]
    max_latency_ms: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum StatusExpectation {
    One(u16),
    Any(Vec<u16>),
}

impl StatusExpectation {
    fn matches(&self, status: u16) -> bool {
        match self {
            Self::One(expected) => *expected == status,
            Self::Any(options) => options.contains(&status),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct McpRules {
    #[serde(default = "default_timeout")]
    timeout_ms: u64,
    #[serde(default = "full_ratio")]
    min_pass_ratio: f64,
    #[serde(default)]
    tools: Vec<ToolRule>,
    #[serde(default)]
    calls: Vec<ToolCall>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolRule {
    name: String,
    #[serde(default)]
    required_params: Vec<String>,
    #[serde(default)]
    min_description_length: Option<usize>,
    /// Rules evaluated against the tool definition from `tools/list`.
    #[serde(default)]
    json: Vec<JsonRule>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolCall {
    id: String,
    label: String,
    tool: String,
    #[serde(default)]
    arguments: Value,
    #[serde(default)]
    expect_error: bool,
    #[serde(default)]
    text_contains_any: Vec<String>,
    /// Rules evaluated against the `tools/call` result.
    #[serde(default)]
    json: Vec<JsonRule>,
    #[serde(default)]
    graded: bool,
}

/// What the browser reports after an MCP session with the learner's server.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct McpRun {
    url: String,
    #[serde(default)]
    nonce: String,
    /// The `initialize` result, absent when the handshake failed.
    #[serde(default)]
    initialize: Option<Value>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    tools: Option<Vec<Value>>,
    #[serde(default)]
    tools_error: Option<String>,
    #[serde(default)]
    calls: Vec<CallRun>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CallRun {
    id: String,
    #[serde(default)]
    result: Option<Value>,
    /// JSON-RPC error object returned instead of a result.
    #[serde(default)]
    rpc_error: Option<Value>,
    #[serde(default)]
    error: Option<String>,
}

fn default_timeout() -> u64 {
    DEFAULT_TIMEOUT_MS
}

fn full_ratio() -> f64 {
    1.0
}

fn get() -> String {
    "GET".into()
}

const METHODS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE"];

fn rule_issues(rules: &[JsonRule], path: &str) -> Vec<String> {
    rules
        .iter()
        .enumerate()
        .flat_map(|(index, rule)| rule.issues(&format!("{path}[{index}]")))
        .collect()
}

fn common_issues(timeout_ms: u64, ratio: f64) -> Vec<String> {
    let mut issues = Vec::new();
    if !(1..=MAX_TIMEOUT_MS).contains(&timeout_ms) {
        issues.push(format!(
            "validation.timeout_ms must be between 1 and {MAX_TIMEOUT_MS}"
        ));
    }
    if !(0.0..=1.0).contains(&ratio) {
        issues.push("validation.min_pass_ratio must be between 0 and 1".into());
    }
    issues
}

fn as_result(issues: Vec<String>) -> Result<(), Vec<String>> {
    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues)
    }
}

fn uses_nonce(value: &impl Serialize) -> bool {
    serde_json::to_string(value).is_ok_and(|text| text.contains(NONCE_TOKEN))
}

#[async_trait]
impl Extension for ServiceExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    fn validate_config(
        &self,
        step_type: &str,
        config: &Value,
        validation: &Value,
    ) -> Result<(), Vec<String>> {
        let config = parse_config::<ServiceConfig>("config", config).map(|_| ());
        match step_type {
            SERVICE_CHECK => collect_issues([
                config,
                parse_config::<Scenario>("validation", validation)
                    .and_then(|scenario| as_result(self.scenario_issues(&scenario))),
            ]),
            MCP_SERVER_CHECK => collect_issues([
                config,
                parse_config::<McpRules>("validation", validation).and_then(|rules| {
                    let mut issues = common_issues(rules.timeout_ms, rules.min_pass_ratio);
                    if rules.tools.is_empty() && rules.calls.is_empty() {
                        issues.push("validation needs tools or calls".into());
                    }
                    for (index, tool) in rules.tools.iter().enumerate() {
                        issues.extend(rule_issues(
                            &tool.json,
                            &format!("validation.tools[{index}].json"),
                        ));
                    }
                    for (index, call) in rules.calls.iter().enumerate() {
                        issues.extend(rule_issues(
                            &call.json,
                            &format!("validation.calls[{index}].json"),
                        ));
                    }
                    as_result(issues)
                }),
            ]),
            other => Err(unsupported_step_type(ID, other)),
        }
    }

    fn client_task(&self, step_type: &str, config: &Value, validation: &Value) -> Option<Value> {
        let config: ServiceConfig = parse_config("config", config).ok()?;
        match step_type {
            SERVICE_CHECK => {
                let scenario: Scenario = parse_config("validation", validation).ok()?;
                Some(json!({
                    "kind": "http",
                    "default_url": config.default_url,
                    "timeout_ms": scenario.timeout_ms,
                    "parallel": scenario.parallel,
                    "cases": scenario.cases.iter().map(|case| json!({
                        "id": case.id,
                        "label": case.label,
                        "request": case.request,
                    })).collect::<Vec<_>>(),
                }))
            }
            MCP_SERVER_CHECK => {
                let rules: McpRules = parse_config("validation", validation).ok()?;
                Some(json!({
                    "kind": "mcp",
                    "default_url": config.default_url,
                    "path": config.path.unwrap_or_else(|| "/mcp".into()),
                    "timeout_ms": rules.timeout_ms,
                    "calls": rules.calls.iter().map(|call| json!({
                        "id": call.id,
                        "label": call.label,
                        "tool": call.tool,
                        "arguments": call.arguments,
                    })).collect::<Vec<_>>(),
                }))
            }
            _ => None,
        }
    }

    fn submission_schema(&self, _step_type: &str, _config: &Value) -> Option<Value> {
        Some(json!({
            "type": "object",
            "description": "Produced by the web app after running client_task in the learner's browser.",
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
        let unavailable = |issues: Vec<String>| ExtensionError::Unavailable(issues.join("; "));
        match ctx.step_type {
            SERVICE_CHECK => {
                let scenario: Scenario =
                    parse_config("validation", ctx.validation).map_err(unavailable)?;
                let run = parse_submission::<HttpRun>(&submission.payload)?.checked()?;
                if uses_nonce(
                    &scenario
                        .cases
                        .iter()
                        .map(|c| &c.request)
                        .collect::<Vec<_>>(),
                ) {
                    check_nonce(&run.nonce)?;
                }
                Ok(evaluate_scenario(&scenario, &run))
            }
            MCP_SERVER_CHECK => {
                let rules: McpRules =
                    parse_config("validation", ctx.validation).map_err(unavailable)?;
                let run: McpRun = parse_submission(&submission.payload)?;
                base_url(&run.url)?;
                if uses_nonce(&rules.calls.iter().map(|c| &c.arguments).collect::<Vec<_>>()) {
                    check_nonce(&run.nonce)?;
                }
                Ok(evaluate_mcp(&rules, &run))
            }
            other => Err(ExtensionError::Unsupported(format!(
                "`{ID}` cannot validate `{other}` steps"
            ))),
        }
    }
}

impl ServiceExtension {
    fn scenario_issues(&self, scenario: &Scenario) -> Vec<String> {
        let mut issues = common_issues(scenario.timeout_ms, scenario.min_pass_ratio);
        if scenario.cases.is_empty() {
            issues.push("validation.cases must not be empty".into());
        }
        let mut ids = HashSet::new();
        for (index, case) in scenario.cases.iter().enumerate() {
            let path = format!("validation.cases[{index}]");
            if !ids.insert(case.id.as_str()) {
                issues.push(format!("{path}.id `{}` is duplicated", case.id));
            }
            if !METHODS.contains(&case.request.method.to_ascii_uppercase().as_str()) {
                issues.push(format!(
                    "{path}.request.method must be GET, POST, PUT, PATCH or DELETE"
                ));
            }
            if !case.request.path.starts_with('/') {
                issues.push(format!("{path}.request.path must start with `/`"));
            }
            issues.extend(rule_issues(
                &case.expect.json,
                &format!("{path}.expect.json"),
            ));
            if let Some(body) = &case.request.json {
                for name in asset_names(body) {
                    if let Err(issue) = self.check_asset(&name) {
                        issues.push(format!("{path}: {issue}"));
                    }
                }
            }
        }
        issues
    }

    /// The browser fetches assets from the API; this only verifies they exist.
    fn check_asset(&self, name: &str) -> Result<(), String> {
        if !is_asset_name(name) {
            return Err(format!("asset `{name}` has an invalid name"));
        }
        let dir = self
            .options
            .assets_dir
            .as_ref()
            .ok_or_else(|| format!("asset `{name}` needs an assets directory"))?;
        match std::fs::metadata(dir.join(name)) {
            Ok(meta) if meta.is_file() && meta.len() <= MAX_ASSET_BYTES => Ok(()),
            Ok(_) => Err(format!("asset `{name}` must be a file of at most 2 MiB")),
            Err(_) => Err(format!("asset `{name}` not found in {}", dir.display())),
        }
    }
}

/// Names of files under `courses/assets` that may be served to browsers.
pub fn is_asset_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 100
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

fn asset_names(value: &Value) -> Vec<String> {
    match value {
        Value::String(text) => text
            .strip_prefix(ASSET_PREFIX)
            .and_then(|rest| rest.strip_suffix("}}"))
            .map(|name| vec![name.to_owned()])
            .unwrap_or_default(),
        Value::Array(items) => items.iter().flat_map(asset_names).collect(),
        Value::Object(map) => map.values().flat_map(asset_names).collect(),
        _ => Vec::new(),
    }
}

struct CaseResult {
    check: CheckResult,
    graded: bool,
}

fn outcome(
    results: Vec<CaseResult>,
    min_ratio: f64,
    passed: &str,
    failed: &str,
) -> ValidationOutcome {
    let total = results.len().max(1);
    let mandatory_failures = results
        .iter()
        .filter(|r| !r.graded && !r.check.passed)
        .count();
    let graded: Vec<&CaseResult> = results.iter().filter(|r| r.graded).collect();
    let graded_passed = graded.iter().filter(|r| r.check.passed).count();
    let ratio_ok =
        graded.is_empty() || graded_passed as f64 / graded.len() as f64 + f64::EPSILON >= min_ratio;
    let passed_count = results.iter().filter(|r| r.check.passed).count();
    let ok = mandatory_failures == 0 && ratio_ok;
    let summary = if ok && passed_count == results.len() {
        passed.to_owned()
    } else if ok {
        format!(
            "{passed} ({passed_count} of {} checks passed)",
            results.len()
        )
    } else if mandatory_failures == 0 {
        format!(
            "{failed}: {graded_passed} of {} quality checks passed, {} needed",
            graded.len(),
            (min_ratio * graded.len() as f64).ceil()
        )
    } else {
        format!(
            "{failed} ({} of {} checks failed)",
            results.len() - passed_count,
            results.len()
        )
    };
    let base = if ok {
        ValidationOutcome::passed(summary)
    } else {
        ValidationOutcome::failed(summary)
    };
    base.with_score(passed_count as f64 / total as f64)
        .with_checks(results.into_iter().map(|r| r.check).collect())
}

fn evaluate_scenario(scenario: &Scenario, run: &HttpRun) -> ValidationOutcome {
    if run.unreachable() {
        return unreachable_outcome(&run.url);
    }
    let results = scenario
        .cases
        .iter()
        .map(|case| {
            let verdict = match run.result(&case.id) {
                None => Err("not run".to_owned()),
                Some(result) => check_case(case, result, &run.nonce),
            };
            CaseResult {
                check: match verdict {
                    Ok(detail) => CheckResult::new(case.id.clone(), case.label.clone(), true)
                        .with_detail(detail),
                    Err(reason) => CheckResult::new(case.id.clone(), case.label.clone(), false)
                        .with_detail(reason),
                },
                graded: case.graded,
            }
        })
        .collect();
    outcome(
        results,
        scenario.min_pass_ratio,
        "Your service passed the hidden checks",
        "Your service did not pass yet",
    )
}

fn check_case(case: &Case, result: &CaseRun, nonce: &str) -> Result<String, String> {
    if let Some(error) = &result.error {
        return Err(describe_error(error));
    }
    let status = result.status.ok_or("no response")?;
    let status_ok = match &case.expect.status {
        Some(expected) => expected.matches(status),
        None => (200..300).contains(&status),
    };
    if !status_ok {
        return Err(format!("HTTP {status}"));
    }
    if let (Some(limit), Some(latency)) = (case.expect.max_latency_ms, result.latency_ms)
        && latency > limit
    {
        return Err(format!("took {latency} ms, limit {limit} ms"));
    }
    if !case.expect.json.is_empty() {
        let document = result
            .body
            .as_deref()
            .and_then(|body| serde_json::from_str::<Value>(body).ok())
            .ok_or("response is not valid JSON")?;
        for rule in &case.expect.json {
            rule.check(&document, nonce)?;
        }
    }
    Ok(result
        .latency_ms
        .map(|ms| format!("{ms} ms"))
        .unwrap_or_default())
}

fn evaluate_mcp(rules: &McpRules, run: &McpRun) -> ValidationOutcome {
    let Some(init) = &run.initialize else {
        let reason = run
            .error
            .as_deref()
            .map_or_else(|| "no response".into(), describe_error);
        return ValidationOutcome::failed(format!(
            "Could not connect to the MCP server at {}",
            run.url
        ))
        .with_checks(vec![
            CheckResult::new("initialize", "Completes the MCP handshake", false)
                .with_detail(reason),
        ]);
    };
    let server_name = init
        .pointer("/serverInfo/name")
        .and_then(Value::as_str)
        .unwrap_or("server");
    let version = init
        .get("protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or("?");
    let mut results = vec![CaseResult {
        check: CheckResult::new("initialize", "Completes the MCP handshake", true)
            .with_detail(format!("{server_name}, protocol {version}")),
        graded: false,
    }];

    let tools = run.tools.as_deref().unwrap_or_default();
    results.push(CaseResult {
        check: match (&run.tools, &run.tools_error) {
            (Some(tools), _) => CheckResult::new("tools-list", "Lists its tools", true)
                .with_detail(format!("{} tools", tools.len())),
            (None, error) => CheckResult::new("tools-list", "Lists its tools", false).with_detail(
                error
                    .as_deref()
                    .map_or_else(|| "no response".into(), describe_error),
            ),
        },
        graded: false,
    });

    for rule in &rules.tools {
        let id = format!("tool-{}", rule.name);
        let label = format!("Offers a well-described `{}` tool", rule.name);
        let found = tools
            .iter()
            .find(|tool| tool.get("name").and_then(Value::as_str) == Some(rule.name.as_str()));
        let check = match found.map(|tool| check_tool(rule, tool)) {
            None => CheckResult::new(id, label, false).with_detail("not listed"),
            Some(Ok(())) => CheckResult::new(id, label, true),
            Some(Err(reason)) => CheckResult::new(id, label, false).with_detail(reason),
        };
        results.push(CaseResult {
            check,
            graded: false,
        });
    }

    for call in &rules.calls {
        let reply = run.calls.iter().find(|reply| reply.id == call.id);
        let verdict = match reply {
            None => Err("not run".to_owned()),
            Some(CallRun {
                error: Some(error), ..
            }) => Err(describe_error(error)),
            Some(CallRun {
                rpc_error: Some(_), ..
            }) if call.expect_error => Ok(()),
            Some(CallRun {
                rpc_error: Some(error),
                ..
            }) => Err(format!(
                "protocol error: {}",
                error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown error")
            )),
            Some(CallRun {
                result: Some(result),
                ..
            }) => check_call(call, result, &run.nonce),
            Some(_) => Err("no result".to_owned()),
        };
        let check = match verdict {
            Ok(()) => CheckResult::new(call.id.clone(), call.label.clone(), true),
            Err(reason) => {
                CheckResult::new(call.id.clone(), call.label.clone(), false).with_detail(reason)
            }
        };
        results.push(CaseResult {
            check,
            graded: call.graded,
        });
    }
    outcome(
        results,
        rules.min_pass_ratio,
        "Your MCP server passed the hidden checks",
        "Your MCP server did not pass yet",
    )
}

fn check_tool(rule: &ToolRule, tool: &Value) -> Result<(), String> {
    let schema = tool.get("inputSchema").ok_or("has no inputSchema")?;
    if schema.get("type").and_then(Value::as_str) != Some("object") {
        return Err("inputSchema.type must be \"object\"".into());
    }
    let required: Vec<&str> = schema
        .get("required")
        .and_then(Value::as_array)
        .map(|list| list.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    for param in &rule.required_params {
        if schema.pointer(&format!("/properties/{param}")).is_none() {
            return Err(format!("parameter `{param}` is not described"));
        }
        if !required.contains(&param.as_str()) {
            return Err(format!("parameter `{param}` is not required"));
        }
    }
    if let Some(min) = rule.min_description_length {
        let length = tool
            .get("description")
            .and_then(Value::as_str)
            .map_or(0, |d| d.trim().chars().count());
        if length < min {
            return Err(format!(
                "description is {length} characters; write at least {min}"
            ));
        }
    }
    for json_rule in &rule.json {
        json_rule.check(tool, "")?;
    }
    Ok(())
}

fn check_call(call: &ToolCall, result: &Value, nonce: &str) -> Result<(), String> {
    let is_error = result
        .get("isError")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if is_error != call.expect_error {
        return Err(if call.expect_error {
            "should return a tool error (isError: true)".into()
        } else {
            format!(
                "returned an error: {}",
                content_text(result).chars().take(120).collect::<String>()
            )
        });
    }
    if !call.text_contains_any.is_empty() {
        let text = format!(
            "{} {}",
            content_text(result),
            result
                .get("structuredContent")
                .map(Value::to_string)
                .unwrap_or_default()
        )
        .to_lowercase();
        if !call
            .text_contains_any
            .iter()
            .any(|needle| text.contains(&needle.to_lowercase()))
        {
            return Err(format!(
                "result does not mention “{}”",
                call.text_contains_any[0]
            ));
        }
    }
    for rule in &call.json {
        rule.check(result, nonce)?;
    }
    Ok(())
}

fn content_text(result: &Value) -> String {
    result
        .get("content")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use lp_core::{OutcomeStatus, SubmissionSource};

    use super::*;

    fn ctx<'a>(step_type: &'a str, validation: &'a Value, config: &'a Value) -> StepContext<'a> {
        StepContext {
            user_id: "u",
            session_id: "s",
            course_id: "c",
            step_id: "step",
            step_type,
            config,
            validation,
        }
    }

    async fn check(
        step_type: &str,
        validation: Value,
        payload: Value,
    ) -> Result<ValidationOutcome, ExtensionError> {
        let config = json!({});
        let extension = ServiceExtension::new(ServiceOptions::default());
        extension
            .validate_config(step_type, &config, &validation)
            .unwrap();
        extension
            .validate_submission(
                &ctx(step_type, &validation, &config),
                &Submission {
                    source: SubmissionSource::Web,
                    payload,
                },
            )
            .await
    }

    fn search_case(id: &str, category: &str) -> Value {
        json!({
            "id": id, "label": id, "graded": true,
            "request": { "path": "/search", "query": { "q": "money" } },
            "expect": { "json": [{ "pointer": "/results", "top_k": 1, "field": "/category", "includes": category }] }
        })
    }

    fn response(id: &str, body: Value) -> Value {
        json!({ "id": id, "status": 200, "latency_ms": 12, "body": body.to_string() })
    }

    #[tokio::test]
    async fn graded_cases_pass_at_the_required_ratio() {
        let finance = json!({ "results": [{ "id": "d1", "category": "finance" }] });
        let payload = json!({ "url": "http://localhost:3000", "results": [response("a", finance.clone()), response("b", finance)] });
        let lenient = json!({ "min_pass_ratio": 0.5, "cases": [search_case("a", "finance"), search_case("b", "hr")] });
        assert_eq!(
            check(SERVICE_CHECK, lenient, payload.clone())
                .await
                .unwrap()
                .status,
            OutcomeStatus::Passed
        );
        let strict = json!({ "cases": [search_case("a", "finance"), search_case("b", "hr")] });
        let outcome = check(SERVICE_CHECK, strict, payload).await.unwrap();
        assert_eq!(outcome.status, OutcomeStatus::Failed);
        assert_eq!(outcome.checks.len(), 2);
    }

    #[tokio::test]
    async fn reports_unreachable_services_and_missing_nonces() {
        let validation = json!({ "cases": [search_case("a", "finance")] });
        let down = json!({ "url": "http://localhost:3000", "results": [{ "id": "a", "error": "network" }] });
        let outcome = check(SERVICE_CHECK, validation, down).await.unwrap();
        assert_eq!(outcome.checks[0].id, "reachable");

        let probe = json!({ "cases": [{ "id": "a", "label": "a", "request": { "method": "DELETE", "path": "/documents/probe-{{nonce}}" } }] });
        let without =
            json!({ "url": "http://localhost:3000", "results": [{ "id": "a", "status": 204 }] });
        assert!(check(SERVICE_CHECK, probe, without).await.is_err());
    }

    #[tokio::test]
    async fn agents_cannot_submit_browser_results() {
        let validation = json!({ "cases": [search_case("a", "finance")] });
        let config = json!({});
        let error = ServiceExtension::new(ServiceOptions::default())
            .validate_submission(
                &ctx(SERVICE_CHECK, &validation, &config),
                &Submission {
                    source: SubmissionSource::Agent,
                    payload: json!({ "url": "http://localhost:3000" }),
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(error, ExtensionError::InvalidSubmission(_)));
    }

    #[tokio::test]
    async fn checks_an_mcp_session() {
        let validation = json!({
            "tools": [{ "name": "get_book", "required_params": ["isbn"], "min_description_length": 20,
                        "json": [{ "pointer": "/annotations/readOnlyHint", "equals": true }] }],
            "calls": [
                { "id": "found", "label": "found", "tool": "get_book", "arguments": { "isbn": "1" }, "text_contains_any": ["lighthouse"] },
                { "id": "missing", "label": "missing", "tool": "get_book", "arguments": { "isbn": "2" }, "expect_error": true }
            ]
        });
        let payload = json!({
            "url": "http://localhost:3000",
            "initialize": { "protocolVersion": "2025-06-18", "serverInfo": { "name": "books" } },
            "tools": [{
                "name": "get_book",
                "description": "Look up a book in the catalog by its ISBN.",
                "inputSchema": { "type": "object", "properties": { "isbn": { "type": "string" } }, "required": ["isbn"] },
                "annotations": { "readOnlyHint": true }
            }],
            "calls": [
                { "id": "found", "result": { "content": [{ "type": "text", "text": "The Lighthouse Ledger" }] } },
                { "id": "missing", "result": { "content": [{ "type": "text", "text": "No such book" }], "isError": true } }
            ]
        });
        let outcome = check(MCP_SERVER_CHECK, validation, payload).await.unwrap();
        assert!(outcome.is_passed(), "{outcome:?}");
    }

    #[test]
    fn publishes_requests_but_never_expectations() {
        let validation = json!({ "cases": [search_case("a", "finance")] });
        let task = ServiceExtension::new(ServiceOptions::default())
            .client_task(SERVICE_CHECK, &json!({}), &validation)
            .unwrap();
        assert_eq!(task["cases"][0]["request"]["query"]["q"], "money");
        assert!(!task.to_string().contains("finance"));
    }

    #[test]
    fn rejects_bad_scenarios() {
        let extension = ServiceExtension::new(ServiceOptions::default());
        let bad = json!({ "cases": [{ "id": "a", "label": "a", "request": { "method": "TRACE", "path": "x", "json": { "image": "{{asset:../etc}}" } } }] });
        let issues = extension
            .validate_config(SERVICE_CHECK, &json!({}), &bad)
            .unwrap_err();
        assert_eq!(issues.len(), 3, "{issues:?}");
    }
}
