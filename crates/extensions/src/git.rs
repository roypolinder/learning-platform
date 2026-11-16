//! Checks repository state reported by the learner's agent (it runs `git` locally;
//! the platform never executes git on user input). Evidence is plausibility-checked
//! but not cryptographically verified; a sandboxed clone-and-verify worker can
//! replace this without changing course definitions.

use async_trait::async_trait;
use lp_core::{
    Capability, CheckResult, CompletionStrategy, Extension, ExtensionError, ExtensionManifest,
    StepCategory, StepContext, StepTypeSpec, Submission, ValidationOutcome, parse_config,
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::evidence::validate_relative_path;
use crate::support::{parse_submission, unsupported_step_type};

pub const ID: &str = "git";
const STEP_TYPE: &str = "git_repository_check";
const MAX_COMMITS: usize = 200;

pub struct GitExtension {
    manifest: ExtensionManifest,
}

impl GitExtension {
    pub fn new() -> Self {
        Self {
            manifest: ExtensionManifest {
                id: ID,
                name: "Git",
                version: "1.0.0",
                description: "Checks branches, commits and tracked files of a repository.",
                capabilities: vec![
                    Capability::ConfigValidation,
                    Capability::SubmissionValidation,
                ],
                step_types: vec![StepTypeSpec::new(
                    STEP_TYPE,
                    StepCategory::Practical,
                    CompletionStrategy::Submission,
                    "Repository state matches the exercise requirements.",
                )],
                event_kinds: Vec::new(),
            },
        }
    }
}

impl Default for GitExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RepositoryRules {
    #[serde(default)]
    branch: Option<String>,
    #[serde(default)]
    min_commits: Option<usize>,
    /// Every commit subject must start with one of these prefixes, e.g. `feat:`.
    #[serde(default)]
    commit_prefixes: Vec<String>,
    #[serde(default)]
    required_files: Vec<String>,
    #[serde(default)]
    require_clean: bool,
    #[serde(default)]
    require_remote: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RepositoryEvidence {
    branch: String,
    commits: Vec<Commit>,
    #[serde(default)]
    tracked_files: Vec<String>,
    #[serde(default)]
    is_clean: Option<bool>,
    #[serde(default)]
    remotes: Vec<Remote>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Commit {
    hash: String,
    message: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Remote {
    name: String,
    #[allow(dead_code)]
    url: String,
}

#[async_trait]
impl Extension for GitExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    fn validate_config(
        &self,
        step_type: &str,
        _config: &Value,
        validation: &Value,
    ) -> Result<(), Vec<String>> {
        if step_type != STEP_TYPE {
            return Err(unsupported_step_type(ID, step_type));
        }
        let rules: RepositoryRules = parse_config("validation", validation)?;
        let issues: Vec<String> = rules
            .required_files
            .iter()
            .filter(|path| validate_relative_path(path).is_err())
            .map(|path| format!("validation.required_files: `{path}` must be relative"))
            .collect();
        if issues.is_empty() {
            Ok(())
        } else {
            Err(issues)
        }
    }

    fn submission_schema(&self, _step_type: &str, _config: &Value) -> Option<Value> {
        Some(json!({
            "type": "object",
            "required": ["branch", "commits"],
            "description": "Collect with `git branch --show-current`, `git log --format='%H %s'`, `git ls-files`, `git status --porcelain` and `git remote -v`.",
            "properties": {
                "branch": { "type": "string" },
                "commits": { "type": "array", "items": {
                    "type": "object", "required": ["hash", "message"],
                    "properties": { "hash": { "type": "string" }, "message": { "type": "string" } }
                }},
                "tracked_files": { "type": "array", "items": { "type": "string" } },
                "is_clean": { "type": "boolean" },
                "remotes": { "type": "array", "items": {
                    "type": "object", "properties": { "name": { "type": "string" }, "url": { "type": "string" } }
                }}
            }
        }))
    }

    async fn validate_submission(
        &self,
        ctx: &StepContext<'_>,
        submission: &Submission,
    ) -> Result<ValidationOutcome, ExtensionError> {
        let rules: RepositoryRules = parse_config("validation", ctx.validation)
            .map_err(|issues| ExtensionError::Unavailable(issues.join("; ")))?;
        let evidence: RepositoryEvidence = parse_submission(&submission.payload)?;
        if evidence.commits.len() > MAX_COMMITS {
            return Err(ExtensionError::InvalidSubmission(format!(
                "send at most {MAX_COMMITS} commits"
            )));
        }
        Ok(check_repository(&rules, &evidence))
    }
}

fn check_repository(rules: &RepositoryRules, evidence: &RepositoryEvidence) -> ValidationOutcome {
    let mut checks = Vec::new();
    let well_formed = evidence
        .commits
        .iter()
        .all(|commit| is_commit_hash(&commit.hash));
    checks.push(CheckResult::new(
        "hashes",
        "Commit hashes are well-formed",
        well_formed,
    ));

    if let Some(branch) = &rules.branch {
        checks.push(
            CheckResult::new(
                "branch",
                format!("Working on branch `{branch}`"),
                &evidence.branch == branch,
            )
            .with_detail(evidence.branch.clone()),
        );
    }
    if let Some(min) = rules.min_commits {
        checks.push(
            CheckResult::new(
                "commits",
                format!("At least {min} commits"),
                evidence.commits.len() >= min,
            )
            .with_detail(format!("{} commits", evidence.commits.len())),
        );
    }
    if !rules.commit_prefixes.is_empty() {
        let offending: Vec<&str> = evidence
            .commits
            .iter()
            .map(|commit| commit.message.lines().next().unwrap_or_default())
            .filter(|subject| {
                !rules
                    .commit_prefixes
                    .iter()
                    .any(|prefix| subject.starts_with(prefix.as_str()))
            })
            .collect();
        let check = CheckResult::new(
            "prefixes",
            format!(
                "Commit subjects start with {}",
                rules.commit_prefixes.join(", ")
            ),
            offending.is_empty(),
        );
        checks.push(match offending.first() {
            Some(first) => check.with_detail(format!("e.g. \"{first}\"")),
            None => check,
        });
    }
    for (index, path) in rules.required_files.iter().enumerate() {
        checks.push(CheckResult::new(
            format!("file-{index}"),
            format!("`{path}` is committed"),
            evidence.tracked_files.iter().any(|tracked| tracked == path),
        ));
    }
    if rules.require_clean {
        checks.push(CheckResult::new(
            "clean",
            "Working tree is clean",
            evidence.is_clean == Some(true),
        ));
    }
    if rules.require_remote {
        checks.push(CheckResult::new(
            "remote",
            "A remote is configured",
            evidence
                .remotes
                .iter()
                .any(|remote| !remote.name.is_empty()),
        ));
    }
    ValidationOutcome::from_checks(
        checks,
        "Repository meets the requirements",
        "Repository is not there yet",
    )
}

fn is_commit_hash(hash: &str) -> bool {
    (7..=64).contains(&hash.len()) && hash.chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use lp_core::{OutcomeStatus, SubmissionSource};

    use super::*;

    #[tokio::test]
    async fn enforces_commit_conventions() {
        let validation = json!({ "min_commits": 2, "commit_prefixes": ["feat:", "fix:"], "required_files": ["README.md"] });
        let config = json!({});
        let ctx = StepContext {
            user_id: "u",
            session_id: "s",
            course_id: "c",
            step_id: "git",
            step_type: STEP_TYPE,
            config: &config,
            validation: &validation,
        };
        let payload = |second: &str| Submission {
            source: SubmissionSource::Agent,
            payload: json!({
                "branch": "main",
                "commits": [ { "hash": "a1b2c3d", "message": "feat: add server" }, { "hash": "b2c3d4e", "message": second } ],
                "tracked_files": ["README.md", "src/main.rs"]
            }),
        };
        let extension = GitExtension::new();
        assert!(
            extension
                .validate_submission(&ctx, &payload("fix: handle errors"))
                .await
                .unwrap()
                .is_passed()
        );
        let outcome = extension
            .validate_submission(&ctx, &payload("wip"))
            .await
            .unwrap();
        assert_eq!(outcome.status, OutcomeStatus::Failed);
    }

    #[test]
    fn hash_format() {
        assert!(is_commit_hash("0a1b2c3"));
        assert!(!is_commit_hash("xyz1234"));
        assert!(!is_commit_hash("abc"));
    }
}
