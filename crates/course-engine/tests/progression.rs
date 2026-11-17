mod support;

use std::sync::Arc;

use async_trait::async_trait;
use lp_core::{
    CompletionStrategy, Extension, ExtensionError, ExtensionEvent, ExtensionManifest, StepCategory,
    StepContext, StepStatus, StepTypeSpec, Submission, SubmissionSource, ValidationOutcome,
};
use lp_course_engine::{
    CourseEngine, EngineContext, EngineError, ExtensionRegistry, SessionIdentity, TransitionReason,
    parse_definition,
};
use serde_json::{Value, json};

const COURSE: &str = r#"
id: flow
version: 1.0.0
title: Flow
summary: Progression scenarios
description: Exercises the state machine.
category: Testing
difficulty: beginner
estimated_minutes: 10
sections:
  - id: main
    title: Main
    steps:
      - { id: intro, type: explanation, title: Intro, body: Hello }
      - id: quiz
        type: multiple_choice
        title: Quiz
        config:
          prompt: Pick a
          options: [ { id: a, text: A, correct: true }, { id: b, text: B } ]
      - id: remedial
        type: conditional
        title: Review
        config: { when: { step: quiz, score_below: 1.0 } }
      - id: connect
        type: mcp_connection_check
        title: Connect
        validation: { required_tool: get_current_step }
      - id: extra
        type: free_text
        title: Extra
        optional: true
        config: { prompt: Why? }
      - { id: done, type: automatic_completion, title: Done, depends_on: [connect] }
"#;

fn identity() -> SessionIdentity {
    SessionIdentity {
        user_id: "user".into(),
        session_id: "session".into(),
    }
}

fn answer(option: &str) -> Submission {
    Submission {
        source: SubmissionSource::Web,
        payload: json!({ "selected": [option] }),
    }
}

#[tokio::test]
async fn walks_a_course_from_start_to_completion() {
    let engine = support::engine();
    let course = support::compile(&engine, COURSE);
    let context = EngineContext::default();
    let (mut progress, transitions) = engine.start(&course, &context);

    assert_eq!(progress.status("intro"), StepStatus::Available);
    assert_eq!(progress.status("quiz"), StepStatus::Locked);
    assert!(
        transitions
            .iter()
            .any(|t| t.reason == TransitionReason::Unlocked && t.step_id == "intro")
    );

    let error = engine
        .acknowledge(&course, &mut progress, "quiz", &context)
        .unwrap_err();
    assert!(matches!(error, EngineError::UnsupportedAction { .. }));

    engine
        .acknowledge(&course, &mut progress, "intro", &context)
        .unwrap();
    assert_eq!(progress.status("quiz"), StepStatus::Available);

    let wrong = engine
        .evaluate_submission(&course, &progress, &identity(), "quiz", &answer("b"))
        .await
        .unwrap();
    engine
        .apply_outcome(&course, &mut progress, "quiz", wrong, &context)
        .unwrap();
    assert_eq!(progress.status("quiz"), StepStatus::Failed);
    assert_eq!(progress.status("remedial"), StepStatus::Locked);

    let right = engine
        .evaluate_submission(&course, &progress, &identity(), "quiz", &answer("a"))
        .await
        .unwrap();
    let advance = engine
        .apply_outcome(&course, &mut progress, "quiz", right, &context)
        .unwrap();
    assert_eq!(progress.status("quiz"), StepStatus::Completed);
    assert_eq!(progress.state("quiz").unwrap().attempts, 2);
    // A perfect score skips the remedial step.
    assert_eq!(progress.status("remedial"), StepStatus::Skipped);
    assert_eq!(progress.status("connect"), StepStatus::Available);
    assert!(!advance.course_completed);

    let unrelated = ExtensionEvent::new("mcp.tool_called", json!({ "tool": "get_session_status" }));
    let advance = engine.handle_event(&course, &mut progress, &identity(), &unrelated, &context);
    assert!(advance.outcomes.is_empty());

    let call = ExtensionEvent::new(
        "mcp.tool_called",
        json!({ "tool": "get_current_step", "client": "VS Code" }),
    );
    let advance = engine.handle_event(&course, &mut progress, &identity(), &call, &context);
    assert_eq!(progress.status("connect"), StepStatus::Completed);
    assert_eq!(progress.status("done"), StepStatus::Completed);
    assert!(
        advance
            .transitions
            .iter()
            .any(|t| t.step_id == "done" && t.reason == TransitionReason::AutoCompleted)
    );
    // `extra` is optional, so the course is complete without it.
    assert_eq!(progress.status("extra"), StepStatus::Available);
    assert!(advance.course_completed);
}

#[tokio::test]
async fn imperfect_scores_unlock_conditional_review() {
    let engine = support::engine();
    let course = support::compile(
        &engine,
        &COURSE.replace("score_below: 1.0", "score_below: 1.5"),
    );
    let context = EngineContext::default();
    let (mut progress, _) = engine.start(&course, &context);
    engine
        .acknowledge(&course, &mut progress, "intro", &context)
        .unwrap();
    let outcome = engine
        .evaluate_submission(&course, &progress, &identity(), "quiz", &answer("a"))
        .await
        .unwrap();
    engine
        .apply_outcome(&course, &mut progress, "quiz", outcome, &context)
        .unwrap();

    assert_eq!(progress.status("remedial"), StepStatus::Available);
    assert_eq!(progress.status("connect"), StepStatus::Locked);
    engine
        .acknowledge(&course, &mut progress, "remedial", &context)
        .unwrap();
    assert_eq!(progress.status("connect"), StepStatus::Available);
}

#[tokio::test]
async fn locked_steps_reject_submissions() {
    let engine = support::engine();
    let course = support::compile(&engine, COURSE);
    let (progress, _) = engine.start(&course, &EngineContext::default());
    let error = engine
        .evaluate_submission(&course, &progress, &identity(), "quiz", &answer("a"))
        .await
        .unwrap_err();
    assert_eq!(
        error,
        EngineError::StepNotActionable {
            step_id: "quiz".into(),
            status: StepStatus::Locked
        }
    );
}

#[test]
fn prerequisites_resolve_from_learner_history() {
    let engine = support::engine();
    let yaml = r#"
id: advanced
version: 1.0.0
title: Advanced
summary: S
description: D
category: C
difficulty: advanced
estimated_minutes: 5
sections:
  - id: main
    title: Main
    steps:
      - { id: gate, type: prerequisite, title: Gate, config: { courses: [basics] } }
      - { id: body, type: explanation, title: Body, body: Text }
"#;
    let course = support::compile(&engine, yaml);
    let (mut progress, _) = engine.start(&course, &EngineContext::default());
    assert_eq!(progress.status("gate"), StepStatus::Available);
    assert_eq!(progress.status("body"), StepStatus::Locked);

    let context = EngineContext {
        completed_courses: ["basics".to_owned()].into(),
        ..EngineContext::default()
    };
    engine.settle(&course, &mut progress, &context);
    assert_eq!(progress.status("gate"), StepStatus::Completed);
    assert_eq!(progress.status("body"), StepStatus::Available);
}

#[test]
fn project_setup_completes_once_the_learner_has_a_project() {
    let engine = support::engine();
    let yaml = r#"
id: setup
version: 1.0.0
title: Setup
summary: S
description: D
category: C
difficulty: beginner
estimated_minutes: 5
sections:
  - id: s
    title: S
    steps:
      - { id: choose, type: project_setup, title: Choose }
      - { id: next, type: explanation, title: Next, body: Text }
"#;
    let course = support::compile(&engine, yaml);
    let (mut progress, _) = engine.start(&course, &EngineContext::default());
    assert_eq!(progress.status("choose"), StepStatus::Available);
    assert_eq!(progress.status("next"), StepStatus::Locked);

    // Learners open the step before choosing; it must still complete afterwards.
    engine.open_step(&course, &mut progress, "choose").unwrap();
    assert_eq!(progress.status("choose"), StepStatus::InProgress);
    let context = EngineContext {
        has_project: true,
        ..EngineContext::default()
    };
    engine.settle(&course, &mut progress, &context);
    assert_eq!(progress.status("choose"), StepStatus::Completed);
    assert_eq!(progress.status("next"), StepStatus::Available);
}

#[test]
fn reconcile_adds_steps_introduced_by_newer_versions() {
    let engine = support::engine();
    let course = support::compile(&engine, COURSE);
    let (mut progress, _) = engine.start(&course, &EngineContext::default());
    progress.steps.remove("extra");
    assert_eq!(
        engine.reconcile(&course, &mut progress),
        vec!["extra".to_owned()]
    );
    assert_eq!(progress.status("extra"), StepStatus::Locked);
}

struct ScreenshotFree {
    manifest: ExtensionManifest,
}

#[async_trait]
impl Extension for ScreenshotFree {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    fn validate_config(&self, _: &str, _: &Value, validation: &Value) -> Result<(), Vec<String>> {
        validation
            .get("expected")
            .map(|_| ())
            .ok_or_else(|| vec!["validation.expected is required".into()])
    }

    async fn validate_submission(
        &self,
        ctx: &StepContext<'_>,
        submission: &Submission,
    ) -> Result<ValidationOutcome, ExtensionError> {
        if submission.payload.get("answer") == ctx.validation.get("expected") {
            Ok(ValidationOutcome::passed("matches"))
        } else {
            Ok(ValidationOutcome::failed("does not match"))
        }
    }
}

#[tokio::test]
async fn new_extensions_plug_in_without_engine_changes() {
    let mut registry = ExtensionRegistry::new();
    registry
        .register(Arc::new(ScreenshotFree {
            manifest: ExtensionManifest {
                id: "docker",
                name: "Docker",
                version: "0.1.0",
                description: "Test double",
                capabilities: vec![],
                step_types: vec![StepTypeSpec::new(
                    "docker_check",
                    StepCategory::Practical,
                    CompletionStrategy::Submission,
                    "Checks a container",
                )],
                event_kinds: vec![],
            },
        }))
        .unwrap();
    let engine = CourseEngine::new(registry).unwrap();
    let yaml = "id: docker\nversion: 1.0.0\ntitle: T\nsummary: S\ndescription: D\ncategory: C\ndifficulty: beginner\nestimated_minutes: 5\nsections:\n  - id: a\n    title: A\n    steps:\n      - { id: run, type: docker_check, title: Run, validation: { expected: 42 } }\n";
    let course = engine.compile(parse_definition(yaml).unwrap()).unwrap();
    assert_eq!(
        course.step("run").unwrap().extension.as_deref(),
        Some("docker")
    );

    let context = EngineContext::default();
    let (mut progress, _) = engine.start(&course, &context);
    let submission = Submission {
        source: SubmissionSource::Agent,
        payload: json!({ "answer": 42 }),
    };
    let outcome = engine
        .evaluate_submission(&course, &progress, &identity(), "run", &submission)
        .await
        .unwrap();
    let advance = engine
        .apply_outcome(&course, &mut progress, "run", outcome, &context)
        .unwrap();
    assert!(advance.course_completed);
}
