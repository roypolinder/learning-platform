use std::collections::{HashMap, HashSet};

use lp_core::{
    CompletionStrategy, ExtensionEvent, OutcomeStatus, StepContext, StepStatus, Submission,
    ValidationOutcome,
};
use serde::{Deserialize, Serialize};

use crate::builtin::{self, CONDITIONAL, PREREQUISITE};
use crate::compiler;
use crate::course::{Course, CourseStep};
use crate::definition::CourseDefinition;
use crate::error::{CourseValidationError, EngineError};
use crate::registry::{ExtensionRegistry, RegistryError, StepTypeEntry, StepTypeRegistry};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StepState {
    pub status: StepStatus,
    pub attempts: u32,
    pub score: Option<f64>,
    pub last_outcome: Option<ValidationOutcome>,
}

/// Per-session state of every step, keyed by step id.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Progress {
    pub steps: HashMap<String, StepState>,
}

impl Progress {
    pub fn status(&self, step_id: &str) -> StepStatus {
        self.steps
            .get(step_id)
            .map(|state| state.status)
            .unwrap_or(StepStatus::Locked)
    }

    pub fn state(&self, step_id: &str) -> Option<&StepState> {
        self.steps.get(step_id)
    }
}

/// Facts about the learner that derived steps depend on.
#[derive(Debug, Clone, Default)]
pub struct EngineContext {
    pub completed_courses: HashSet<String>,
}

#[derive(Debug, Clone)]
pub struct SessionIdentity {
    pub user_id: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionReason {
    Initialized,
    Unlocked,
    Opened,
    Acknowledged,
    ValidationPassed,
    ValidationFailed,
    ValidationPending,
    ConditionMet,
    ConditionNotMet,
    PrerequisiteMet,
    AutoCompleted,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StepTransition {
    pub step_id: String,
    pub from: Option<StepStatus>,
    pub to: StepStatus,
    pub reason: TransitionReason,
}

#[derive(Debug, Clone, Default)]
pub struct Advance {
    pub transitions: Vec<StepTransition>,
    /// Outcomes recorded during this advance, keyed by step id.
    pub outcomes: Vec<(String, ValidationOutcome)>,
    pub course_completed: bool,
}

pub struct CourseEngine {
    extensions: ExtensionRegistry,
    step_types: StepTypeRegistry,
}

impl CourseEngine {
    pub fn new(extensions: ExtensionRegistry) -> Result<Self, RegistryError> {
        let step_types = StepTypeRegistry::build(&extensions)?;
        Ok(Self {
            extensions,
            step_types,
        })
    }

    pub fn extensions(&self) -> &ExtensionRegistry {
        &self.extensions
    }

    pub fn step_types(&self) -> impl Iterator<Item = &StepTypeEntry> {
        self.step_types.entries()
    }

    pub fn compile(&self, definition: CourseDefinition) -> Result<Course, CourseValidationError> {
        compiler::compile(definition, &self.step_types, &self.extensions)
    }

    pub fn start(
        &self,
        course: &Course,
        context: &EngineContext,
    ) -> (Progress, Vec<StepTransition>) {
        let mut progress = Progress::default();
        let mut transitions = Vec::with_capacity(course.steps.len() * 2);
        for step in &course.steps {
            progress.steps.insert(step.id.clone(), StepState::default());
            transitions.push(StepTransition {
                step_id: step.id.clone(),
                from: None,
                to: StepStatus::Locked,
                reason: TransitionReason::Initialized,
            });
        }
        transitions.extend(self.settle(course, &mut progress, context));
        (progress, transitions)
    }

    /// Adds state for steps missing from persisted progress, e.g. after a course
    /// definition gained steps. Returns the ids that were added.
    pub fn reconcile(&self, course: &Course, progress: &mut Progress) -> Vec<String> {
        let mut added = Vec::new();
        for step in &course.steps {
            if !progress.steps.contains_key(&step.id) {
                progress.steps.insert(step.id.clone(), StepState::default());
                added.push(step.id.clone());
            }
        }
        added
    }

    pub fn open_step(
        &self,
        course: &Course,
        progress: &mut Progress,
        step_id: &str,
    ) -> Result<Vec<StepTransition>, EngineError> {
        let step = find_step(course, step_id)?;
        match progress.status(&step.id) {
            StepStatus::Available => Ok(vec![set_status(
                progress,
                &step.id,
                StepStatus::InProgress,
                TransitionReason::Opened,
            )]),
            StepStatus::Locked => Err(EngineError::StepNotActionable {
                step_id: step.id.clone(),
                status: StepStatus::Locked,
            }),
            _ => Ok(Vec::new()),
        }
    }

    pub fn acknowledge(
        &self,
        course: &Course,
        progress: &mut Progress,
        step_id: &str,
        context: &EngineContext,
    ) -> Result<Advance, EngineError> {
        let step = find_step(course, step_id)?;
        let accepts =
            step.completion == CompletionStrategy::Acknowledge || step.step_type == CONDITIONAL;
        if !accepts {
            return Err(EngineError::UnsupportedAction {
                step_id: step.id.clone(),
                action: "manual completion",
            });
        }
        ensure_actionable(progress, step)?;
        let mut transitions = vec![set_status(
            progress,
            &step.id,
            StepStatus::Completed,
            TransitionReason::Acknowledged,
        )];
        transitions.extend(self.settle(course, progress, context));
        Ok(Advance {
            transitions,
            outcomes: Vec::new(),
            course_completed: is_complete(course, progress),
        })
    }

    /// Runs the step's extension against a submission without mutating progress,
    /// so callers can avoid holding locks during slow validations.
    pub async fn evaluate_submission(
        &self,
        course: &Course,
        progress: &Progress,
        identity: &SessionIdentity,
        step_id: &str,
        submission: &Submission,
    ) -> Result<ValidationOutcome, EngineError> {
        let step = find_step(course, step_id)?;
        if step.completion != CompletionStrategy::Submission {
            return Err(EngineError::UnsupportedAction {
                step_id: step.id.clone(),
                action: "submissions",
            });
        }
        ensure_actionable(progress, step)?;
        let extension_id = step
            .extension
            .as_deref()
            .ok_or_else(|| EngineError::MissingExtension(step.step_type.clone()))?;
        let extension = self
            .extensions
            .get(extension_id)
            .ok_or_else(|| EngineError::MissingExtension(extension_id.to_owned()))?;
        let context = step_context(course, step, identity);
        Ok(extension.validate_submission(&context, submission).await?)
    }

    pub fn apply_outcome(
        &self,
        course: &Course,
        progress: &mut Progress,
        step_id: &str,
        outcome: ValidationOutcome,
        context: &EngineContext,
    ) -> Result<Advance, EngineError> {
        let step = find_step(course, step_id)?;
        ensure_actionable(progress, step)?;
        let mut transitions = record_outcome(progress, &step.id, &outcome);
        transitions.extend(self.settle(course, progress, context));
        Ok(Advance {
            transitions,
            outcomes: vec![(step.id.clone(), outcome)],
            course_completed: is_complete(course, progress),
        })
    }

    /// Offers an event to every actionable step whose extension subscribes to its kind.
    pub fn handle_event(
        &self,
        course: &Course,
        progress: &mut Progress,
        identity: &SessionIdentity,
        event: &ExtensionEvent,
        context: &EngineContext,
    ) -> Advance {
        let mut advance = Advance::default();
        for step in &course.steps {
            if !progress.status(&step.id).is_actionable() {
                continue;
            }
            let Some(extension) = step
                .extension
                .as_deref()
                .and_then(|id| self.extensions.get(id))
            else {
                continue;
            };
            if !extension
                .manifest()
                .event_kinds
                .iter()
                .any(|kind| *kind == event.kind)
            {
                continue;
            }
            let step_context = step_context(course, step, identity);
            if let Some(outcome) = extension.handle_event(&step_context, event) {
                advance
                    .transitions
                    .extend(record_outcome(progress, &step.id, &outcome));
                advance.outcomes.push((step.id.clone(), outcome));
            }
        }
        if !advance.outcomes.is_empty() {
            advance
                .transitions
                .extend(self.settle(course, progress, context));
        }
        advance.course_completed = is_complete(course, progress);
        advance
    }

    /// Unlocks steps whose dependencies are resolved and resolves derived steps,
    /// repeating until nothing changes.
    pub fn settle(
        &self,
        course: &Course,
        progress: &mut Progress,
        context: &EngineContext,
    ) -> Vec<StepTransition> {
        let mut transitions = Vec::new();
        loop {
            let mut changed = false;
            for step in &course.steps {
                let next = match progress.status(&step.id) {
                    StepStatus::Locked if dependencies_resolved(step, progress) => {
                        Some(unlock_target(step, progress))
                    }
                    StepStatus::Available => resolve_available(step, context),
                    _ => None,
                };
                if let Some((status, reason)) = next {
                    transitions.push(set_status(progress, &step.id, status, reason));
                    changed = true;
                }
            }
            if !changed {
                return transitions;
            }
        }
    }

    pub fn is_complete(&self, course: &Course, progress: &Progress) -> bool {
        is_complete(course, progress)
    }

    /// The first step, in course order, the learner can act on. Derived steps wait
    /// on external facts, so they are only suggested when nothing else is open.
    pub fn current_step<'a>(
        &self,
        course: &'a Course,
        progress: &Progress,
    ) -> Option<&'a CourseStep> {
        let mut actionable = course
            .steps
            .iter()
            .filter(|step| progress.status(&step.id).is_actionable());
        let first = actionable.clone().next();
        actionable
            .find(|step| step.completion != CompletionStrategy::Derived)
            .or(first)
    }
}

fn find_step<'a>(course: &'a Course, step_id: &str) -> Result<&'a CourseStep, EngineError> {
    course
        .step(step_id)
        .ok_or_else(|| EngineError::UnknownStep(step_id.to_owned()))
}

fn ensure_actionable(progress: &Progress, step: &CourseStep) -> Result<(), EngineError> {
    let status = progress.status(&step.id);
    if status.is_actionable() {
        Ok(())
    } else {
        Err(EngineError::StepNotActionable {
            step_id: step.id.clone(),
            status,
        })
    }
}

fn step_context<'a>(
    course: &'a Course,
    step: &'a CourseStep,
    identity: &'a SessionIdentity,
) -> StepContext<'a> {
    StepContext {
        user_id: &identity.user_id,
        session_id: &identity.session_id,
        course_id: &course.id,
        step_id: &step.id,
        step_type: &step.step_type,
        config: &step.config,
        validation: &step.validation,
    }
}

fn set_status(
    progress: &mut Progress,
    step_id: &str,
    to: StepStatus,
    reason: TransitionReason,
) -> StepTransition {
    let state = progress.steps.entry(step_id.to_owned()).or_default();
    let from = state.status;
    debug_assert!(
        from.can_transition_to(to),
        "invalid transition {from} -> {to} for {step_id}"
    );
    state.status = to;
    StepTransition {
        step_id: step_id.to_owned(),
        from: Some(from),
        to,
        reason,
    }
}

fn record_outcome(
    progress: &mut Progress,
    step_id: &str,
    outcome: &ValidationOutcome,
) -> Vec<StepTransition> {
    let state = progress.steps.entry(step_id.to_owned()).or_default();
    state.attempts += 1;
    if outcome.score.is_some() {
        state.score = outcome.score;
    }
    state.last_outcome = Some(outcome.clone());
    let current = state.status;
    let (target, reason) = match outcome.status {
        OutcomeStatus::Passed => (StepStatus::Completed, TransitionReason::ValidationPassed),
        OutcomeStatus::Failed => (StepStatus::Failed, TransitionReason::ValidationFailed),
        OutcomeStatus::Pending => (StepStatus::InProgress, TransitionReason::ValidationPending),
    };
    if current == target || !current.can_transition_to(target) {
        return Vec::new();
    }
    vec![set_status(progress, step_id, target, reason)]
}

fn dependencies_resolved(step: &CourseStep, progress: &Progress) -> bool {
    step.depends_on
        .iter()
        .all(|dependency| progress.status(dependency).is_resolved())
}

fn unlock_target(step: &CourseStep, progress: &Progress) -> (StepStatus, TransitionReason) {
    if step.step_type != CONDITIONAL {
        return (StepStatus::Available, TransitionReason::Unlocked);
    }
    let holds = builtin::condition_of(&step.config).is_some_and(|condition| {
        let state = progress.state(&condition.step);
        let score = state.and_then(|state| state.score);
        if let Some(status) = condition.status {
            progress.status(&condition.step) == status
        } else if let Some(threshold) = condition.score_below {
            score.is_some_and(|score| score < threshold)
        } else if let Some(threshold) = condition.score_at_least {
            score.is_some_and(|score| score >= threshold)
        } else {
            false
        }
    });
    if holds {
        (StepStatus::Available, TransitionReason::ConditionMet)
    } else {
        (StepStatus::Skipped, TransitionReason::ConditionNotMet)
    }
}

fn resolve_available(
    step: &CourseStep,
    context: &EngineContext,
) -> Option<(StepStatus, TransitionReason)> {
    match step.completion {
        CompletionStrategy::Automatic => {
            Some((StepStatus::Completed, TransitionReason::AutoCompleted))
        }
        CompletionStrategy::Derived if step.step_type == PREREQUISITE => {
            let met = builtin::prerequisite_courses(&step.config)
                .iter()
                .all(|course| context.completed_courses.contains(course));
            met.then_some((StepStatus::Completed, TransitionReason::PrerequisiteMet))
        }
        _ => None,
    }
}

fn is_complete(course: &Course, progress: &Progress) -> bool {
    course
        .required_steps
        .iter()
        .all(|step_id| progress.status(step_id).is_resolved())
}
