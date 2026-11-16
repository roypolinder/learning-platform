//! Knowledge checks evaluated entirely server-side. Answer keys and explanations
//! are removed from learner-visible configs and only returned as feedback.

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use lp_core::{
    Capability, CheckResult, CompletionStrategy, Extension, ExtensionError, ExtensionManifest,
    StepCategory, StepContext, StepTypeSpec, Submission, ValidationOutcome, parse_config,
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::support::{collect_issues, parse_submission, unsupported_step_type};

pub const ID: &str = "quiz";

pub struct QuizExtension {
    manifest: ExtensionManifest,
}

impl QuizExtension {
    pub fn new() -> Self {
        let assessment = |name, description| {
            StepTypeSpec::new(
                name,
                StepCategory::Assessment,
                CompletionStrategy::Submission,
                description,
            )
        };
        Self {
            manifest: ExtensionManifest {
                id: ID,
                name: "Quiz",
                version: "1.0.0",
                description: "Multiple choice, free text and fill-in exercises.",
                capabilities: vec![
                    Capability::ConfigValidation,
                    Capability::SubmissionValidation,
                    Capability::ConfigRedaction,
                ],
                step_types: vec![
                    assessment("quiz", "Several questions scored together."),
                    assessment("multiple_choice", "A single question with options."),
                    assessment(
                        "free_text",
                        "A written answer checked against rubric rules.",
                    ),
                    assessment("interactive_exercise", "Fill in the blanks of a template."),
                ],
                event_kinds: Vec::new(),
            },
        }
    }
}

impl Default for QuizExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct QuizConfig {
    questions: Vec<Question>,
    #[serde(default = "full_score")]
    pass_score: f64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Question {
    #[serde(default)]
    id: String,
    prompt: String,
    options: Vec<AnswerOption>,
    #[serde(default)]
    multiple: bool,
    #[serde(default)]
    explanation: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AnswerOption {
    id: String,
    text: String,
    #[serde(default)]
    correct: bool,
    #[serde(default)]
    explanation: Option<String>,
}

// Fields are rendered by the frontend; parsing only enforces the shape.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct FreeTextConfig {
    prompt: String,
    #[serde(default)]
    placeholder: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FreeTextRules {
    #[serde(default)]
    min_words: Option<usize>,
    #[serde(default)]
    max_words: Option<usize>,
    #[serde(default)]
    required_concepts: Vec<Concept>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Concept {
    label: String,
    any_of: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExerciseConfig {
    #[allow(dead_code)]
    prompt: String,
    /// Text containing `{{blank-id}}` placeholders.
    template: String,
    blanks: Vec<Blank>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Blank {
    id: String,
    #[serde(default)]
    hint: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExerciseRules {
    answers: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    case_sensitive: bool,
}

fn full_score() -> f64 {
    1.0
}

impl QuizConfig {
    fn from_step(step_type: &str, config: &Value) -> Result<Self, Vec<String>> {
        if step_type == "multiple_choice" {
            let mut question: Question = parse_config("config", config)?;
            question.id = "answer".into();
            return Ok(Self {
                questions: vec![question],
                pass_score: 1.0,
            });
        }
        parse_config("config", config)
    }

    fn check(&self) -> Result<(), Vec<String>> {
        let mut issues = Vec::new();
        if self.questions.is_empty() {
            issues.push("config.questions must not be empty".into());
        }
        if !(0.0..=1.0).contains(&self.pass_score) {
            issues.push("config.pass_score must be between 0 and 1".into());
        }
        let mut ids = BTreeSet::new();
        for question in &self.questions {
            if question.id.is_empty() || !ids.insert(question.id.as_str()) {
                issues.push(format!(
                    "question ids must be unique and non-empty (`{}`)",
                    question.id
                ));
            }
            if question.prompt.trim().is_empty() {
                issues.push(format!("question `{}` needs a prompt", question.id));
            }
            let correct = question
                .options
                .iter()
                .filter(|option| option.correct)
                .count();
            if question.options.len() < 2 {
                issues.push(format!(
                    "question `{}` needs at least two options",
                    question.id
                ));
            }
            if correct == 0 || (!question.multiple && correct > 1) {
                issues.push(format!(
                    "question `{}` must have {} correct option",
                    question.id,
                    if question.multiple {
                        "at least one"
                    } else {
                        "exactly one"
                    }
                ));
            }
            let mut option_ids = BTreeSet::new();
            for option in &question.options {
                if !option_ids.insert(option.id.as_str()) {
                    issues.push(format!(
                        "question `{}` repeats option `{}`",
                        question.id, option.id
                    ));
                }
            }
        }
        if issues.is_empty() {
            Ok(())
        } else {
            Err(issues)
        }
    }
}

#[async_trait]
impl Extension for QuizExtension {
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
            "quiz" | "multiple_choice" => {
                QuizConfig::from_step(step_type, config).and_then(|quiz| quiz.check())
            }
            "free_text" => collect_issues([
                parse_config::<FreeTextConfig>("config", config).map(|_| ()),
                parse_config::<FreeTextRules>("validation", validation).map(|_| ()),
            ]),
            "interactive_exercise" => {
                let exercise = parse_config::<ExerciseConfig>("config", config)?;
                let rules = parse_config::<ExerciseRules>("validation", validation)?;
                let mut issues = Vec::new();
                for blank in &exercise.blanks {
                    if !exercise.template.contains(&format!("{{{{{}}}}}", blank.id)) {
                        issues.push(format!("blank `{}` is not used in the template", blank.id));
                    }
                    if rules.answers.get(&blank.id).is_none_or(Vec::is_empty) {
                        issues.push(format!("blank `{}` has no accepted answers", blank.id));
                    }
                }
                if issues.is_empty() {
                    Ok(())
                } else {
                    Err(issues)
                }
            }
            other => Err(unsupported_step_type(ID, other)),
        }
    }

    fn public_config(&self, step_type: &str, config: &Value) -> Value {
        let mut public = config.clone();
        let strip = |question: &mut Value| {
            if let Some(object) = question.as_object_mut() {
                object.remove("explanation");
            }
            if let Some(options) = question.get_mut("options").and_then(Value::as_array_mut) {
                for option in options.iter_mut().filter_map(Value::as_object_mut) {
                    option.remove("correct");
                    option.remove("explanation");
                }
            }
        };
        match step_type {
            "quiz" => {
                if let Some(questions) = public.get_mut("questions").and_then(Value::as_array_mut) {
                    questions.iter_mut().for_each(strip);
                }
            }
            "multiple_choice" => strip(&mut public),
            _ => {}
        }
        public
    }

    fn submission_schema(&self, step_type: &str, _config: &Value) -> Option<Value> {
        let schema = match step_type {
            "quiz" => json!({
                "type": "object",
                "required": ["answers"],
                "properties": { "answers": {
                    "type": "object",
                    "description": "Question id -> selected option ids",
                    "additionalProperties": { "type": "array", "items": { "type": "string" } }
                }}
            }),
            "multiple_choice" => json!({
                "type": "object",
                "required": ["selected"],
                "properties": { "selected": { "type": "array", "items": { "type": "string" } } }
            }),
            "free_text" => json!({
                "type": "object",
                "required": ["text"],
                "properties": { "text": { "type": "string" } }
            }),
            "interactive_exercise" => json!({
                "type": "object",
                "required": ["blanks"],
                "properties": { "blanks": {
                    "type": "object",
                    "additionalProperties": { "type": "string" }
                }}
            }),
            _ => return None,
        };
        Some(schema)
    }

    async fn validate_submission(
        &self,
        ctx: &StepContext<'_>,
        submission: &Submission,
    ) -> Result<ValidationOutcome, ExtensionError> {
        let invalid_config = |issues: Vec<String>| ExtensionError::Unavailable(issues.join("; "));
        match ctx.step_type {
            "quiz" => {
                #[derive(Deserialize)]
                struct Answers {
                    answers: BTreeMap<String, Vec<String>>,
                }
                let quiz = QuizConfig::from_step("quiz", ctx.config).map_err(invalid_config)?;
                let answers: Answers = parse_submission(&submission.payload)?;
                Ok(grade(&quiz, &answers.answers))
            }
            "multiple_choice" => {
                #[derive(Deserialize)]
                struct Selected {
                    selected: Vec<String>,
                }
                let quiz =
                    QuizConfig::from_step("multiple_choice", ctx.config).map_err(invalid_config)?;
                let selected: Selected = parse_submission(&submission.payload)?;
                Ok(grade(
                    &quiz,
                    &BTreeMap::from([("answer".to_owned(), selected.selected)]),
                ))
            }
            "free_text" => {
                #[derive(Deserialize)]
                struct Text {
                    text: String,
                }
                let rules = parse_config::<FreeTextRules>("validation", ctx.validation)
                    .map_err(invalid_config)?;
                let text: Text = parse_submission(&submission.payload)?;
                Ok(grade_free_text(&rules, &text.text))
            }
            "interactive_exercise" => {
                #[derive(Deserialize)]
                struct Blanks {
                    blanks: BTreeMap<String, String>,
                }
                let exercise =
                    parse_config::<ExerciseConfig>("config", ctx.config).map_err(invalid_config)?;
                let rules = parse_config::<ExerciseRules>("validation", ctx.validation)
                    .map_err(invalid_config)?;
                let blanks: Blanks = parse_submission(&submission.payload)?;
                Ok(grade_exercise(&exercise, &rules, &blanks.blanks))
            }
            other => Err(ExtensionError::Unsupported(format!("`{other}` steps"))),
        }
    }
}

fn grade(quiz: &QuizConfig, answers: &BTreeMap<String, Vec<String>>) -> ValidationOutcome {
    let mut checks = Vec::with_capacity(quiz.questions.len());
    let mut feedback = Vec::with_capacity(quiz.questions.len());
    for question in &quiz.questions {
        let expected: BTreeSet<&str> = question
            .options
            .iter()
            .filter(|option| option.correct)
            .map(|option| option.id.as_str())
            .collect();
        let given: BTreeSet<&str> = answers
            .get(&question.id)
            .map(|ids| ids.iter().map(String::as_str).collect())
            .unwrap_or_default();
        let correct = expected == given;
        checks.push(CheckResult::new(
            question.id.clone(),
            question.prompt.clone(),
            correct,
        ));
        let option_notes: Vec<Value> = question
            .options
            .iter()
            .filter(|option| given.contains(option.id.as_str()))
            .filter_map(|option| {
                option.explanation.as_ref().map(
                    |note| json!({ "option": option.id, "text": option.text, "explanation": note }),
                )
            })
            .collect();
        feedback.push(json!({
            "question": question.id,
            "correct": correct,
            "explanation": question.explanation,
            "selected": option_notes,
        }));
    }
    let correct = checks.iter().filter(|check| check.passed).count();
    let score = correct as f64 / quiz.questions.len().max(1) as f64;
    let outcome = if score + f64::EPSILON >= quiz.pass_score {
        ValidationOutcome::passed(format!("{correct} of {} correct", quiz.questions.len()))
    } else {
        ValidationOutcome::failed(format!(
            "{correct} of {} correct; {:.0}% needed",
            quiz.questions.len(),
            quiz.pass_score * 100.0
        ))
    };
    outcome
        .with_score(score)
        .with_checks(checks)
        .with_feedback(json!({ "questions": feedback }))
}

fn grade_free_text(rules: &FreeTextRules, text: &str) -> ValidationOutcome {
    let words = text.split_whitespace().count();
    let lowered = text.to_lowercase();
    let mut checks = Vec::new();
    if let Some(min) = rules.min_words {
        checks.push(
            CheckResult::new("min_words", format!("At least {min} words"), words >= min)
                .with_detail(format!("{words} words")),
        );
    }
    if let Some(max) = rules.max_words {
        checks.push(
            CheckResult::new("max_words", format!("At most {max} words"), words <= max)
                .with_detail(format!("{words} words")),
        );
    }
    for (index, concept) in rules.required_concepts.iter().enumerate() {
        let mentioned = concept
            .any_of
            .iter()
            .any(|term| lowered.contains(&term.to_lowercase()));
        checks.push(CheckResult::new(
            format!("concept-{index}"),
            format!("Mentions {}", concept.label),
            mentioned,
        ));
    }
    if text.trim().is_empty() {
        checks.push(CheckResult::new("non_empty", "Answer is not empty", false));
    }
    ValidationOutcome::from_checks(
        checks,
        "Answer meets every criterion",
        "Answer is missing something",
    )
}

fn grade_exercise(
    exercise: &ExerciseConfig,
    rules: &ExerciseRules,
    given: &BTreeMap<String, String>,
) -> ValidationOutcome {
    let normalize = |value: &str| {
        let trimmed = value.trim();
        if rules.case_sensitive {
            trimmed.to_owned()
        } else {
            trimmed.to_lowercase()
        }
    };
    let checks: Vec<CheckResult> = exercise
        .blanks
        .iter()
        .map(|blank| {
            let answer = given
                .get(&blank.id)
                .map(|value| normalize(value))
                .unwrap_or_default();
            let accepted = rules.answers.get(&blank.id).is_some_and(|accepted| {
                accepted
                    .iter()
                    .any(|candidate| normalize(candidate) == answer)
            });
            let check =
                CheckResult::new(blank.id.clone(), format!("Blank `{}`", blank.id), accepted);
            match (&blank.hint, accepted) {
                (Some(hint), false) => check.with_detail(hint.clone()),
                _ => check,
            }
        })
        .collect();
    let correct = checks.iter().filter(|check| check.passed).count();
    let score = correct as f64 / checks.len().max(1) as f64;
    ValidationOutcome::from_checks(
        checks,
        "All blanks are correct",
        "Some blanks are not right yet",
    )
    .with_score(score)
}

#[cfg(test)]
mod tests {
    use lp_core::{OutcomeStatus, SubmissionSource};

    use super::*;

    fn context<'a>(
        step_type: &'a str,
        config: &'a Value,
        validation: &'a Value,
    ) -> StepContext<'a> {
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

    fn submission(payload: Value) -> Submission {
        Submission {
            source: SubmissionSource::Web,
            payload,
        }
    }

    fn quiz_config() -> Value {
        json!({
            "pass_score": 0.5,
            "questions": [
                { "id": "q1", "prompt": "Transport?", "explanation": "Streamable HTTP.",
                  "options": [ { "id": "a", "text": "HTTP", "correct": true }, { "id": "b", "text": "FTP" } ] },
                { "id": "q2", "prompt": "Pick primitives", "multiple": true,
                  "options": [ { "id": "t", "text": "Tools", "correct": true },
                               { "id": "r", "text": "Resources", "correct": true },
                               { "id": "x", "text": "Threads" } ] }
            ]
        })
    }

    #[test]
    fn public_config_hides_answer_keys() {
        let public = QuizExtension::new().public_config("quiz", &quiz_config());
        let text = public.to_string();
        assert!(!text.contains("correct"));
        assert!(!text.contains("Streamable HTTP."));
        assert_eq!(public["questions"][0]["options"][0]["text"], "HTTP");
    }

    #[test]
    fn rejects_questions_without_a_correct_option() {
        let config = json!({ "questions": [ { "id": "q", "prompt": "?",
            "options": [ { "id": "a", "text": "A" }, { "id": "b", "text": "B" } ] } ] });
        let issues = QuizExtension::new()
            .validate_config("quiz", &config, &Value::Null)
            .unwrap_err();
        assert!(issues[0].contains("exactly one correct option"));
    }

    #[tokio::test]
    async fn grades_quiz_against_pass_score() {
        let extension = QuizExtension::new();
        let config = quiz_config();
        let ctx = context("quiz", &config, &Value::Null);

        let partial = extension
            .validate_submission(
                &ctx,
                &submission(json!({ "answers": { "q1": ["a"], "q2": ["t"] } })),
            )
            .await
            .unwrap();
        assert_eq!(partial.status, OutcomeStatus::Passed);
        assert_eq!(partial.score, Some(0.5));

        let wrong = extension
            .validate_submission(&ctx, &submission(json!({ "answers": { "q1": ["b"] } })))
            .await
            .unwrap();
        assert_eq!(wrong.status, OutcomeStatus::Failed);
        assert_eq!(wrong.score, Some(0.0));
    }

    #[tokio::test]
    async fn free_text_requires_concepts() {
        let config = json!({ "prompt": "Explain" });
        let validation = json!({ "min_words": 3, "required_concepts": [ { "label": "tools", "any_of": ["tool"] } ] });
        let ctx = context("free_text", &config, &validation);
        let extension = QuizExtension::new();

        let missing = extension
            .validate_submission(
                &ctx,
                &submission(json!({ "text": "servers expose resources" })),
            )
            .await
            .unwrap();
        assert_eq!(missing.status, OutcomeStatus::Failed);

        let ok = extension
            .validate_submission(
                &ctx,
                &submission(json!({ "text": "Servers expose Tools to agents" })),
            )
            .await
            .unwrap();
        assert!(ok.is_passed());
    }

    #[tokio::test]
    async fn exercise_accepts_alternative_answers_case_insensitively() {
        let config = json!({ "prompt": "Fill", "template": "Use {{verb}} over {{transport}}",
            "blanks": [ { "id": "verb" }, { "id": "transport", "hint": "The modern transport" } ] });
        let validation =
            json!({ "answers": { "verb": ["POST"], "transport": ["streamable http", "http"] } });
        let ctx = context("interactive_exercise", &config, &validation);
        let outcome = QuizExtension::new()
            .validate_submission(
                &ctx,
                &submission(json!({ "blanks": { "verb": "post", "transport": "HTTP" } })),
            )
            .await
            .unwrap();
        assert!(outcome.is_passed());
    }
}
