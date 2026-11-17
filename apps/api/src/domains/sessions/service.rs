//! Orchestrates the course engine with persistence. Every state change runs in a
//! transaction holding a row lock on the session, so concurrent requests from the
//! browser and an agent cannot interleave transitions.

use std::sync::Arc;

use chrono::Utc;
use lp_core::{
    ExtensionEvent, SessionStatus, StepStatus, Submission, SubmissionSource, ValidationOutcome,
};
use lp_course_engine::{
    Advance, Course, CourseEngine, EngineContext, EngineError, Progress, SessionIdentity,
};
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use super::domain::{Actor, CourseSession, NewEvent, progress_from_rows};
use super::repository;
use crate::domains::courses::catalog::LoadedCourse;
use crate::domains::learners::domain::LearnerProfile;
use crate::domains::learners::repository as learners;
use crate::error::{ApiError, ApiResult};
use crate::realtime::SessionUpdate;
use crate::state::AppState;

pub struct SessionSnapshot {
    pub session: CourseSession,
    pub course: Arc<LoadedCourse>,
    pub progress: Progress,
}

pub struct MutationResult {
    pub snapshot: SessionSnapshot,
    pub outcome: Option<ValidationOutcome>,
}

pub struct SessionService<'a> {
    state: &'a AppState,
}

impl<'a> SessionService<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    fn engine(&self) -> &CourseEngine {
        &self.state.engine
    }

    /// A course version in the learner's language with their project filled in.
    async fn course_for(
        &self,
        version_id: Uuid,
        profile: &LearnerProfile,
    ) -> ApiResult<Arc<LoadedCourse>> {
        let template = self
            .state
            .catalog
            .version(&self.state.db, self.engine(), version_id)
            .await?;
        self.render(&template, profile)
    }

    fn render(
        &self,
        template: &LoadedCourse,
        profile: &LearnerProfile,
    ) -> ApiResult<Arc<LoadedCourse>> {
        self.state.catalog.render(
            self.engine(),
            template,
            profile.audience(&self.state.themes),
        )
    }

    /// Starts a course, or resumes the learner's active session for it.
    pub async fn start(
        &self,
        user_id: Uuid,
        course_slug: &str,
    ) -> ApiResult<(SessionSnapshot, bool)> {
        let loaded = self
            .state
            .catalog
            .current(course_slug)
            .ok_or(ApiError::NotFound("course"))?;
        if let Some(existing) =
            repository::find_active(&self.state.db, user_id, loaded.course_uuid).await?
        {
            if let Some(upgraded) = self.upgrade_if_outdated(user_id, &existing).await? {
                return Ok((upgraded, false));
            }
            let snapshot = self.refresh(user_id, existing.id, Actor::Learner).await?;
            return Ok((snapshot, false));
        }
        match self.create(user_id, &loaded, None).await? {
            Some(session_id) => Ok((self.load(user_id, session_id).await?, true)),
            None => {
                // Lost a race with a concurrent start; the other request created it.
                let existing = repository::find_active(&self.state.db, user_id, loaded.course_uuid)
                    .await?
                    .ok_or_else(|| ApiError::internal("active session vanished"))?;
                Ok((self.load(user_id, existing.id).await?, false))
            }
        }
    }

    /// Moves an active session on an outdated course version to the current version, so
    /// learners never keep following superseded instructions. Completed steps carry over
    /// when their content is unchanged; the old session is kept as abandoned.
    pub async fn upgrade_if_outdated(
        &self,
        user_id: Uuid,
        session: &CourseSession,
    ) -> ApiResult<Option<SessionSnapshot>> {
        if session.status != SessionStatus::Active {
            return Ok(None);
        }
        let Some(loaded) = self.state.catalog.current(&session.course_slug) else {
            return Ok(None);
        };
        if loaded.version_id == session.course_version_id {
            return Ok(None);
        }
        let previous = match self.load(user_id, session.id).await {
            Ok(previous) => Some(previous),
            Err(error) => {
                // Versions from before a breaking engine change can no longer be compiled.
                // Their progress cannot carry over, so the learner starts the current version.
                tracing::warn!(session = %session.id, %error, "outdated session cannot be loaded; abandoning it");
                self.abandon(user_id, session.id).await?;
                None
            }
        };
        let session_id = match self.create(user_id, &loaded, previous.as_ref()).await? {
            Some(id) => id,
            None => {
                match repository::find_active(&self.state.db, user_id, loaded.course_uuid).await? {
                    Some(active) => active.id,
                    None => return Ok(None),
                }
            }
        };
        tracing::info!(from = %session.id, to = %session_id, course = %loaded.course.id, "session upgraded to current course version");
        Ok(Some(self.load(user_id, session_id).await?))
    }

    async fn abandon(&self, user_id: Uuid, session_id: Uuid) -> ApiResult<()> {
        let mut tx = self.state.db.begin().await?;
        if let Some(locked) = repository::lock_for_user(&mut tx, session_id, user_id).await?
            && locked.status == SessionStatus::Active
        {
            repository::update_state(
                &mut tx,
                locked.id,
                SessionStatus::Abandoned,
                locked.current_step_id.as_deref(),
                None,
            )
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// Returns `None` when another request created the active session first.
    async fn create(
        &self,
        user_id: Uuid,
        loaded: &LoadedCourse,
        previous: Option<&SessionSnapshot>,
    ) -> ApiResult<Option<Uuid>> {
        let mut tx = self.state.db.begin().await?;
        if let Some(previous) = previous {
            let locked = repository::lock_for_user(&mut tx, previous.session.id, user_id)
                .await?
                .ok_or(ApiError::NotFound("session"))?;
            if locked.status != SessionStatus::Active {
                tx.rollback().await?;
                return Ok(None);
            }
            repository::update_state(
                &mut tx,
                locked.id,
                SessionStatus::Abandoned,
                locked.current_step_id.as_deref(),
                None,
            )
            .await?;
        }
        let (context, profile) = learner_context(&mut tx, user_id).await?;
        let loaded = &*self.render(loaded, &profile)?;
        let (mut progress, transitions) = self.engine().start(&loaded.course, &context);
        let carried = match previous {
            Some(previous) => carry_over(
                &previous.course.course,
                &previous.progress,
                &loaded.course,
                &mut progress,
            ),
            None => Vec::new(),
        };
        if !carried.is_empty() {
            self.engine()
                .settle(&loaded.course, &mut progress, &context);
        }
        let session_id = Uuid::now_v7();
        let inserted = repository::insert(
            &mut tx,
            session_id,
            user_id,
            loaded.course_uuid,
            loaded.version_id,
        )
        .await?;
        if !inserted {
            tx.rollback().await?;
            return Ok(None);
        }

        let steps: Vec<(&str, &lp_course_engine::StepState)> = loaded
            .course
            .steps
            .iter()
            .filter_map(|step| {
                progress
                    .state(&step.id)
                    .map(|state| (step.id.as_str(), state))
            })
            .collect();
        repository::insert_steps(&mut tx, session_id, &steps).await?;
        for step_id in &carried {
            if let Some(state) = progress.state(step_id) {
                repository::update_step(&mut tx, session_id, step_id, state).await?;
            }
        }

        let mut events = vec![NewEvent {
            step_id: None,
            kind: "session.started",
            payload: json!({
                "course": loaded.course.id,
                "version": loaded.course.version,
                "upgraded_from": previous.map(|p| p.session.id),
                "carried_steps": carried,
            }),
        }];
        events.extend(transitions.iter().filter(|t| t.from.is_some()).map(|transition| NewEvent {
            step_id: Some(transition.step_id.clone()),
            kind: "step.transition",
            payload: json!({ "from": transition.from, "to": transition.to, "reason": transition.reason, "actor": "system" }),
        }));
        repository::insert_events(&mut tx, session_id, &events).await?;

        let current = self
            .engine()
            .current_step(&loaded.course, &progress)
            .map(|step| step.id.as_str());
        let (status, completed_at) = if self.engine().is_complete(&loaded.course, &progress) {
            (SessionStatus::Completed, Some(Utc::now()))
        } else {
            (SessionStatus::Active, None)
        };
        repository::update_state(&mut tx, session_id, status, current, completed_at).await?;
        tx.commit().await?;
        tracing::info!(%session_id, %user_id, course = %loaded.course.id, "course session started");
        Ok(Some(session_id))
    }

    pub async fn load(&self, user_id: Uuid, session_id: Uuid) -> ApiResult<SessionSnapshot> {
        let session = repository::find_for_user(&self.state.db, session_id, user_id)
            .await?
            .ok_or(ApiError::NotFound("session"))?;
        let profile = learners::profile(&self.state.db, user_id).await?;
        let course = self.course_for(session.course_version_id, &profile).await?;
        let mut progress =
            progress_from_rows(repository::load_steps(&self.state.db, session.id).await?);
        self.engine().reconcile(&course.course, &mut progress);
        Ok(SessionSnapshot {
            session,
            course,
            progress,
        })
    }

    pub async fn touch(&self, session_id: Uuid) -> ApiResult<()> {
        repository::touch(&self.state.db, session_id).await?;
        Ok(())
    }

    /// Session agents act on when they don't name one: the most recently used active session.
    pub async fn resolve_for_agent(
        &self,
        user_id: Uuid,
        session_id: Option<Uuid>,
    ) -> ApiResult<SessionSnapshot> {
        let session_id = match session_id {
            Some(id) => id,
            None => repository::most_recent_active(&self.state.db, user_id)
                .await?
                .map(|session| session.id)
                .ok_or(ApiError::NotFound("active session"))?,
        };
        let snapshot = self.load(user_id, session_id).await?;
        match self.upgrade_if_outdated(user_id, &snapshot.session).await? {
            Some(upgraded) => Ok(upgraded),
            None => Ok(snapshot),
        }
    }

    pub async fn open_step(
        &self,
        user_id: Uuid,
        session_id: Uuid,
        step_id: &str,
    ) -> ApiResult<MutationResult> {
        self.mutate(
            user_id,
            session_id,
            Actor::Learner,
            |engine, course, progress, _| {
                Ok(Advance {
                    transitions: engine.open_step(course, progress, step_id)?,
                    ..Advance::default()
                })
            },
        )
        .await
    }

    pub async fn complete_step(
        &self,
        user_id: Uuid,
        session_id: Uuid,
        step_id: &str,
    ) -> ApiResult<MutationResult> {
        self.mutate(
            user_id,
            session_id,
            Actor::Learner,
            |engine, course, progress, context| {
                engine.acknowledge(course, progress, step_id, context)
            },
        )
        .await
    }

    /// Validation can be slow (network checks), so it runs before the session lock
    /// is taken; the outcome is then applied against freshly locked state.
    pub async fn submit(
        &self,
        user_id: Uuid,
        session_id: Uuid,
        step_id: &str,
        submission: Submission,
    ) -> ApiResult<MutationResult> {
        let snapshot = self.load(user_id, session_id).await?;
        ensure_open(&snapshot.session)?;
        let identity = SessionIdentity {
            user_id: user_id.to_string(),
            session_id: session_id.to_string(),
        };
        let outcome = self
            .engine()
            .evaluate_submission(
                &snapshot.course.course,
                &snapshot.progress,
                &identity,
                step_id,
                &submission,
            )
            .await?;
        let actor = match submission.source {
            SubmissionSource::Web => Actor::Learner,
            SubmissionSource::Agent => Actor::Agent,
        };
        self.mutate(
            user_id,
            session_id,
            actor,
            move |engine, course, progress, context| {
                engine.apply_outcome(course, progress, step_id, outcome, context)
            },
        )
        .await
    }

    pub async fn dispatch_event(
        &self,
        user_id: Uuid,
        session_id: Uuid,
        event: ExtensionEvent,
    ) -> ApiResult<MutationResult> {
        let identity = SessionIdentity {
            user_id: user_id.to_string(),
            session_id: session_id.to_string(),
        };
        self.mutate(
            user_id,
            session_id,
            Actor::Agent,
            move |engine, course, progress, context| {
                Ok(engine.handle_event(course, progress, &identity, &event, context))
            },
        )
        .await
    }

    /// Re-evaluates derived steps, e.g. prerequisites satisfied by another course.
    pub async fn refresh(
        &self,
        user_id: Uuid,
        session_id: Uuid,
        actor: Actor,
    ) -> ApiResult<SessionSnapshot> {
        let result = self
            .mutate(
                user_id,
                session_id,
                actor,
                |engine, course, progress, context| {
                    let transitions = engine.settle(course, progress, context);
                    Ok(Advance {
                        course_completed: engine.is_complete(course, progress),
                        transitions,
                        outcomes: Vec::new(),
                    })
                },
            )
            .await;
        match result {
            Ok(result) => Ok(result.snapshot),
            Err(ApiError::Conflict { .. }) => self.load(user_id, session_id).await,
            Err(error) => Err(error),
        }
    }

    async fn mutate<F>(
        &self,
        user_id: Uuid,
        session_id: Uuid,
        actor: Actor,
        operation: F,
    ) -> ApiResult<MutationResult>
    where
        F: FnOnce(
            &CourseEngine,
            &Course,
            &mut Progress,
            &EngineContext,
        ) -> Result<Advance, EngineError>,
    {
        let mut tx = self.state.db.begin().await?;
        let mut session = repository::lock_for_user(&mut tx, session_id, user_id)
            .await?
            .ok_or(ApiError::NotFound("session"))?;
        ensure_open(&session)?;
        let (context, profile) = learner_context(&mut tx, user_id).await?;
        let course = self.course_for(session.course_version_id, &profile).await?;
        let mut progress = progress_from_rows(repository::load_steps(&mut *tx, session.id).await?);
        let added = self.engine().reconcile(&course.course, &mut progress);
        if !added.is_empty() {
            let rows: Vec<_> = added
                .iter()
                .filter_map(|id| progress.state(id).map(|state| (id.as_str(), state)))
                .collect();
            repository::insert_steps(&mut tx, session.id, &rows).await?;
        }
        let before = progress.clone();

        let advance = operation(self.engine(), &course.course, &mut progress, &context)?;

        let mut changed: Vec<&String> = progress
            .steps
            .iter()
            .filter(|(id, state)| before.steps.get(*id) != Some(state))
            .map(|(id, _)| id)
            .collect();
        changed.sort();
        for step_id in &changed {
            if let Some(state) = progress.state(step_id) {
                repository::update_step(&mut tx, session.id, step_id, state).await?;
            }
        }

        let mut events: Vec<NewEvent> = advance
            .outcomes
            .iter()
            .map(|(step_id, outcome)| NewEvent {
                step_id: Some(step_id.clone()),
                kind: "step.validated",
                payload: json!({
                    "status": outcome.status,
                    "summary": outcome.summary,
                    "score": outcome.score,
                    "actor": actor.as_str(),
                }),
            })
            .collect();
        events.extend(advance.transitions.iter().map(|transition| NewEvent {
            step_id: Some(transition.step_id.clone()),
            kind: "step.transition",
            payload: json!({
                "from": transition.from,
                "to": transition.to,
                "reason": transition.reason,
                "actor": actor.as_str(),
            }),
        }));

        let newly_completed = advance.course_completed && session.status == SessionStatus::Active;
        if newly_completed {
            session.status = session
                .status
                .transition_to(SessionStatus::Completed)
                .map_err(ApiError::internal)?;
            session.completed_at = Some(Utc::now());
            events.push(NewEvent {
                step_id: None,
                kind: "session.completed",
                payload: json!({ "actor": actor.as_str() }),
            });
        }
        repository::insert_events(&mut tx, session.id, &events).await?;

        session.current_step_id = self
            .engine()
            .current_step(&course.course, &progress)
            .map(|step| step.id.clone());
        repository::update_state(
            &mut tx,
            session.id,
            session.status,
            session.current_step_id.as_deref(),
            session.completed_at,
        )
        .await?;
        tx.commit().await?;

        if !changed.is_empty() || newly_completed {
            self.state.hub.publish(SessionUpdate {
                session_id: session.id,
                user_id,
                kind: "progress",
                step_ids: changed.iter().map(|id| (*id).clone()).collect(),
                course_completed: newly_completed,
            });
        }
        let outcome = advance
            .outcomes
            .into_iter()
            .next()
            .map(|(_, outcome)| outcome);
        Ok(MutationResult {
            snapshot: SessionSnapshot {
                session,
                course,
                progress,
            },
            outcome,
        })
    }
}

/// Copies completed steps whose content (body, config, rules) is identical in both versions.
fn carry_over(
    old: &Course,
    old_progress: &Progress,
    new: &Course,
    progress: &mut Progress,
) -> Vec<String> {
    let mut carried = Vec::new();
    for step in &new.steps {
        let unchanged = old.step(&step.id).is_some_and(|before| {
            before.step_type == step.step_type
                && before.body == step.body
                && before.config == step.config
                && before.validation == step.validation
        });
        let Some(state) = old_progress.state(&step.id) else {
            continue;
        };
        if unchanged && state.status == StepStatus::Completed {
            progress.steps.insert(step.id.clone(), state.clone());
            carried.push(step.id.clone());
        }
    }
    carried
}

fn ensure_open(session: &CourseSession) -> ApiResult<()> {
    if session.status == SessionStatus::Abandoned {
        return Err(ApiError::Conflict {
            code: "session_abandoned",
            message: "This session was abandoned. Start the course again to continue.".into(),
        });
    }
    Ok(())
}

async fn learner_context(
    connection: &mut PgConnection,
    user_id: Uuid,
) -> ApiResult<(EngineContext, LearnerProfile)> {
    let profile = learners::profile(&mut *connection, user_id).await?;
    let mut context = EngineContext {
        completed_courses: repository::completed_course_slugs(connection, user_id)
            .await?
            .into_iter()
            .collect(),
        ..EngineContext::default()
    };
    profile.apply_to(&mut context);
    Ok((context, profile))
}
