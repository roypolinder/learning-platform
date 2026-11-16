//! Validates file trees and configuration documents submitted as evidence.

use async_trait::async_trait;
use lp_core::{
    Capability, CheckResult, CompletionStrategy, Extension, ExtensionError, ExtensionManifest,
    StepCategory, StepContext, StepTypeSpec, Submission, ValidationOutcome, parse_config,
};
use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::evidence::{FileEvidence, SubmittedFile, path_matches};
use crate::support::{collect_issues, parse_submission, unsupported_step_type};

pub const ID: &str = "filesystem";
const FILESYSTEM_VALIDATION: &str = "filesystem_validation";
const CONFIGURATION_VALIDATION: &str = "configuration_validation";
/// Generic engine step types that may delegate to this extension.
const GENERIC_TYPES: &[&str] = &["extension_validation", "external_task"];

pub struct FilesystemExtension {
    manifest: ExtensionManifest,
}

impl FilesystemExtension {
    pub fn new() -> Self {
        let practical = |name, description| {
            StepTypeSpec::new(
                name,
                StepCategory::Practical,
                CompletionStrategy::Submission,
                description,
            )
        };
        Self {
            manifest: ExtensionManifest {
                id: ID,
                name: "Filesystem",
                version: "1.0.0",
                description: "Checks files and configuration documents created by the learner.",
                capabilities: vec![
                    Capability::ConfigValidation,
                    Capability::SubmissionValidation,
                ],
                step_types: vec![
                    practical(
                        FILESYSTEM_VALIDATION,
                        "Required files exist with the expected content.",
                    ),
                    practical(
                        CONFIGURATION_VALIDATION,
                        "A JSON or YAML document has the expected values.",
                    ),
                ],
                event_kinds: Vec::new(),
            },
        }
    }
}

impl Default for FilesystemExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileRules {
    files: Vec<FileRule>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileRule {
    path: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    min_length: Option<usize>,
    #[serde(default)]
    max_length: Option<usize>,
    #[serde(default)]
    contains: Vec<String>,
    #[serde(default)]
    frontmatter: Option<FrontmatterRule>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FrontmatterRule {
    #[serde(default)]
    required: Vec<String>,
    #[serde(default)]
    max_length: std::collections::BTreeMap<String, usize>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DocumentConfig {
    file: String,
    format: DocumentFormat,
    #[serde(default)]
    #[allow(dead_code)]
    instructions: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum DocumentFormat {
    Json,
    Yaml,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DocumentRules {
    rules: Vec<DocumentRule>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DocumentRule {
    /// JSON pointer; a `*` segment matches any key or index.
    pointer: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    equals: Option<Value>,
    #[serde(default)]
    contains: Option<String>,
    #[serde(default)]
    one_of: Vec<Value>,
    /// Passes only when the pointer matches nothing, e.g. "no hardcoded credentials".
    #[serde(default)]
    absent: bool,
}

#[async_trait]
impl Extension for FilesystemExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    fn validate_config(
        &self,
        step_type: &str,
        config: &Value,
        validation: &Value,
    ) -> Result<(), Vec<String>> {
        match step_type {
            FILESYSTEM_VALIDATION => file_rules(validation).map(|_| ()),
            CONFIGURATION_VALIDATION => collect_issues([
                parse_config::<DocumentConfig>("config", config).map(|_| ()),
                parse_config::<DocumentRules>("validation", validation).and_then(|rules| {
                    let issues: Vec<String> = rules
                        .rules
                        .iter()
                        .filter(|rule| !rule.pointer.starts_with('/'))
                        .map(|rule| format!("`{}` must be a JSON pointer", rule.pointer))
                        .collect();
                    if issues.is_empty() {
                        Ok(())
                    } else {
                        Err(issues)
                    }
                }),
            ]),
            generic if GENERIC_TYPES.contains(&generic) => file_rules(validation).map(|_| ()),
            other => Err(unsupported_step_type(ID, other)),
        }
    }

    fn submission_schema(&self, _step_type: &str, _config: &Value) -> Option<Value> {
        Some(json!({
            "type": "object",
            "required": ["files"],
            "properties": { "files": {
                "type": "array",
                "maxItems": crate::evidence::MAX_FILES,
                "items": {
                    "type": "object",
                    "required": ["path", "content"],
                    "properties": {
                        "path": { "type": "string", "description": "Path relative to the project root" },
                        "content": { "type": "string" }
                    }
                }
            }}
        }))
    }

    async fn validate_submission(
        &self,
        ctx: &StepContext<'_>,
        submission: &Submission,
    ) -> Result<ValidationOutcome, ExtensionError> {
        let evidence = parse_submission::<FileEvidence>(&submission.payload)?.checked()?;
        let unavailable = |issues: Vec<String>| ExtensionError::Unavailable(issues.join("; "));
        match ctx.step_type {
            CONFIGURATION_VALIDATION => {
                let config: DocumentConfig =
                    parse_config("config", ctx.config).map_err(unavailable)?;
                let rules: DocumentRules =
                    parse_config("validation", ctx.validation).map_err(unavailable)?;
                Ok(check_document(&config, &rules, &evidence.files))
            }
            _ => Ok(check_files(
                &file_rules(ctx.validation).map_err(unavailable)?,
                &evidence.files,
            )),
        }
    }
}

fn file_rules(validation: &Value) -> Result<FileRules, Vec<String>> {
    let rules: FileRules = parse_config("validation", validation)?;
    if rules.files.is_empty() {
        return Err(vec!["validation.files must not be empty".into()]);
    }
    Ok(rules)
}

fn check_files(rules: &FileRules, files: &[SubmittedFile]) -> ValidationOutcome {
    let mut checks = Vec::new();
    for (index, rule) in rules.files.iter().enumerate() {
        let label = rule
            .label
            .clone()
            .unwrap_or_else(|| format!("`{}`", rule.path));
        let id = |suffix: &str| format!("file-{index}-{suffix}");
        let Some(file) = files
            .iter()
            .find(|file| path_matches(&rule.path, &file.path))
        else {
            checks.push(CheckResult::new(
                id("exists"),
                format!("{label} exists"),
                false,
            ));
            continue;
        };
        checks.push(
            CheckResult::new(id("exists"), format!("{label} exists"), true)
                .with_detail(file.path.clone()),
        );
        let length = file.content.chars().count();
        if let Some(min) = rule.min_length {
            checks.push(
                CheckResult::new(
                    id("min"),
                    format!("{label} has at least {min} characters"),
                    length >= min,
                )
                .with_detail(format!("{length} characters")),
            );
        }
        if let Some(max) = rule.max_length {
            checks.push(
                CheckResult::new(
                    id("max"),
                    format!("{label} has at most {max} characters"),
                    length <= max,
                )
                .with_detail(format!("{length} characters")),
            );
        }
        for (needle_index, needle) in rule.contains.iter().enumerate() {
            checks.push(CheckResult::new(
                id(&format!("contains-{needle_index}")),
                format!("{label} contains `{needle}`"),
                file.content.contains(needle.as_str()),
            ));
        }
        if let Some(frontmatter_rule) = &rule.frontmatter {
            checks.extend(check_frontmatter(
                &id("frontmatter"),
                &label,
                frontmatter_rule,
                &file.content,
            ));
        }
    }
    ValidationOutcome::from_checks(checks, "All files look right", "Some files need work")
}

fn check_frontmatter(
    id: &str,
    label: &str,
    rule: &FrontmatterRule,
    content: &str,
) -> Vec<CheckResult> {
    let parsed = frontmatter(content);
    let mut checks = vec![CheckResult::new(
        id.to_owned(),
        format!("{label} starts with YAML frontmatter"),
        parsed.is_some(),
    )];
    let fields = parsed.unwrap_or_default();
    for field in &rule.required {
        let present = fields.get(field).is_some_and(|value| {
            !value.is_null() && value.as_str().is_none_or(|text| !text.trim().is_empty())
        });
        checks.push(CheckResult::new(
            format!("{id}-{field}"),
            format!("Frontmatter defines `{field}`"),
            present,
        ));
    }
    for (field, max) in &rule.max_length {
        let length = fields
            .get(field)
            .and_then(Value::as_str)
            .map_or(0, |text| text.chars().count());
        checks.push(
            CheckResult::new(
                format!("{id}-{field}-length"),
                format!("`{field}` is at most {max} characters"),
                length <= *max,
            )
            .with_detail(format!("{length} characters")),
        );
    }
    checks
}

fn frontmatter(content: &str) -> Option<Map<String, Value>> {
    let rest = content.trim_start_matches('\u{feff}').strip_prefix("---")?;
    let rest = rest
        .strip_prefix("\r\n")
        .or_else(|| rest.strip_prefix('\n'))?;
    let end = rest.find("\n---")?;
    match serde_saphyr::from_str::<Value>(&rest[..end]).ok()? {
        Value::Object(map) => Some(map),
        _ => None,
    }
}

fn check_document(
    config: &DocumentConfig,
    rules: &DocumentRules,
    files: &[SubmittedFile],
) -> ValidationOutcome {
    let Some(file) = files
        .iter()
        .find(|file| path_matches(&config.file, &file.path))
    else {
        return ValidationOutcome::failed(format!("`{}` was not submitted", config.file))
            .with_checks(vec![CheckResult::new(
                "exists",
                format!("`{}` exists", config.file),
                false,
            )]);
    };
    let document = match config.format {
        DocumentFormat::Json => {
            serde_json::from_str::<Value>(&file.content).map_err(|e| e.to_string())
        }
        DocumentFormat::Yaml => {
            serde_saphyr::from_str::<Value>(&file.content).map_err(|e| e.to_string())
        }
    };
    let document = match document {
        Ok(document) => document,
        Err(error) => {
            return ValidationOutcome::failed(format!("`{}` could not be parsed", file.path))
                .with_checks(vec![
                    CheckResult::new("parse", "Document parses", false).with_detail(error),
                ]);
        }
    };

    let mut checks = vec![CheckResult::new("parse", "Document parses", true)];
    for (index, rule) in rules.rules.iter().enumerate() {
        let candidates = resolve_pointer(&document, &rule.pointer);
        let passed = if rule.absent {
            candidates.is_empty()
        } else {
            candidates.iter().any(|value| {
                rule.equals
                    .as_ref()
                    .is_none_or(|expected| *value == expected)
                    && rule.contains.as_ref().is_none_or(|needle| {
                        value
                            .as_str()
                            .is_some_and(|text| text.contains(needle.as_str()))
                    })
                    && (rule.one_of.is_empty() || rule.one_of.contains(value))
            })
        };
        let label = rule.label.clone().unwrap_or_else(|| describe_rule(rule));
        checks.push(CheckResult::new(format!("rule-{index}"), label, passed));
    }
    ValidationOutcome::from_checks(
        checks,
        "Configuration is valid",
        "Configuration needs changes",
    )
}

fn describe_rule(rule: &DocumentRule) -> String {
    if rule.absent {
        return format!("`{}` is not set", rule.pointer);
    }
    match (&rule.equals, &rule.contains) {
        (Some(expected), _) => format!("`{}` is {expected}", rule.pointer),
        (None, Some(needle)) => format!("`{}` contains `{needle}`", rule.pointer),
        _ => format!("`{}` is set", rule.pointer),
    }
}

fn resolve_pointer<'a>(document: &'a Value, pointer: &str) -> Vec<&'a Value> {
    let mut current = vec![document];
    for raw in pointer.split('/').skip(1) {
        let segment = raw.replace("~1", "/").replace("~0", "~");
        current = current
            .into_iter()
            .flat_map(|value| -> Vec<&Value> {
                match (value, segment.as_str()) {
                    (Value::Object(map), "*") => map.values().collect(),
                    (Value::Array(items), "*") => items.iter().collect(),
                    (Value::Object(map), key) => map.get(key).into_iter().collect(),
                    (Value::Array(items), index) => index
                        .parse::<usize>()
                        .ok()
                        .and_then(|index| items.get(index))
                        .into_iter()
                        .collect(),
                    _ => Vec::new(),
                }
            })
            .collect();
    }
    current
}

#[cfg(test)]
mod tests {
    use lp_core::{OutcomeStatus, SubmissionSource};

    use super::*;

    fn ctx<'a>(step_type: &'a str, config: &'a Value, validation: &'a Value) -> StepContext<'a> {
        StepContext {
            user_id: "u",
            session_id: "s",
            course_id: "c",
            step_id: "x",
            step_type,
            config,
            validation,
        }
    }

    fn files(entries: &[(&str, &str)]) -> Submission {
        let files: Vec<Value> = entries
            .iter()
            .map(|(path, content)| json!({ "path": path, "content": content }))
            .collect();
        Submission {
            source: SubmissionSource::Agent,
            payload: json!({ "files": files }),
        }
    }

    #[tokio::test]
    async fn validates_skill_frontmatter_and_sections() {
        let validation = json!({ "files": [ {
            "path": "**/SKILL.md", "min_length": 40, "contains": ["## Steps"],
            "frontmatter": { "required": ["name", "description"], "max_length": { "description": 200 } }
        } ] });
        let config = json!({});
        let extension = FilesystemExtension::new();
        let good = "---\nname: review\ndescription: Reviews pull requests\n---\n# Review\n\n## Steps\n1. Read the diff\n";
        let outcome = extension
            .validate_submission(
                &ctx(FILESYSTEM_VALIDATION, &config, &validation),
                &files(&[(".github/skills/review/SKILL.md", good)]),
            )
            .await
            .unwrap();
        assert!(outcome.is_passed(), "{outcome:?}");

        let missing = "# Review\n\n## Steps\nNo frontmatter here at all, sorry.";
        let outcome = extension
            .validate_submission(
                &ctx(FILESYSTEM_VALIDATION, &config, &validation),
                &files(&[("SKILL.md", missing)]),
            )
            .await
            .unwrap();
        assert_eq!(outcome.status, OutcomeStatus::Failed);
        assert!(
            outcome
                .checks
                .iter()
                .any(|check| check.label.contains("frontmatter") && !check.passed)
        );
    }

    #[tokio::test]
    async fn validates_json_configuration_with_wildcards() {
        let config = json!({ "file": ".vscode/mcp.json", "format": "json" });
        let validation = json!({ "rules": [
            { "pointer": "/servers/*/type", "equals": "http" },
            { "pointer": "/servers/*/url", "contains": "/mcp" }
        ] });
        let document = r#"{ "servers": { "learning": { "type": "http", "url": "http://localhost:8080/mcp" } } }"#;
        let outcome = FilesystemExtension::new()
            .validate_submission(
                &ctx(CONFIGURATION_VALIDATION, &config, &validation),
                &files(&[(".vscode/mcp.json", document)]),
            )
            .await
            .unwrap();
        assert!(outcome.is_passed(), "{outcome:?}");
    }

    #[tokio::test]
    async fn absent_rules_fail_when_any_match_exists() {
        let config = json!({ "file": ".mcp.json", "format": "json" });
        let validation = json!({ "rules": [
            { "pointer": "/mcpServers/*/headers/Authorization", "absent": true }
        ] });
        let check = |document: &'static str| {
            let config = config.clone();
            let validation = validation.clone();
            async move {
                FilesystemExtension::new()
                    .validate_submission(
                        &ctx(CONFIGURATION_VALIDATION, &config, &validation),
                        &files(&[(".mcp.json", document)]),
                    )
                    .await
                    .unwrap()
                    .is_passed()
            }
        };
        assert!(check(r#"{ "mcpServers": { "a": { "type": "http" } } }"#).await);
        assert!(
            !check(
                r#"{ "mcpServers": { "a": {}, "b": { "headers": { "Authorization": "x" } } } }"#
            )
            .await
        );
    }

    #[tokio::test]
    async fn rejects_path_traversal() {
        let validation = json!({ "files": [ { "path": "a.txt" } ] });
        let error = FilesystemExtension::new()
            .validate_submission(
                &ctx(FILESYSTEM_VALIDATION, &json!({}), &validation),
                &files(&[("../a.txt", "x")]),
            )
            .await
            .unwrap_err();
        assert!(matches!(error, ExtensionError::InvalidSubmission(_)));
    }
}
