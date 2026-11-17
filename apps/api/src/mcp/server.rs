use http::request::Parts;
use lp_core::{ExtensionEvent, Submission, SubmissionSource};
use lp_course_engine::view::step_view;
use lp_extensions::mcp::TOOL_CALLED;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::tool::Extension;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Implementation, ServerCapabilities, ServerConfig};
use rmcp::{ErrorData, ServerHandler, schemars, tool, tool_handler, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domains::authoring::service as authoring;
use crate::domains::integrations::domain::McpPrincipal;
use crate::domains::integrations::repository as connections;
use crate::domains::learners::repository as learners;
use crate::domains::sessions::dto::ProgressDto;
use crate::domains::sessions::service::{SessionService, SessionSnapshot};
use crate::error::ApiError;
use crate::state::AppState;

const INSTRUCTIONS: &str = "This server belongs to the learning platform the user is taking courses on. \
The learner builds one website throughout the course track: call get_project for its theme, the \
learner's chosen programming language and the seed data the site loads. \
Call get_current_step to see what the learner should do next, including the evidence format. \
When the learner has finished a practical task, collect the requested evidence from their workspace \
and send it with report_task_result. Never invent evidence; report what is actually in the workspace. \
Super admins additionally get authoring_* tools to create and update courses: read authoring_get_guide first, \
check drafts with authoring_validate_course and only call authoring_publish_course when the user asks to publish.";

const AUTHORING_PREFIX: &str = "authoring_";

#[derive(Clone)]
pub struct LearningPlatformServer {
    state: AppState,
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SessionArgs {
    /// Course session id. Defaults to the learner's most recently used active session.
    #[serde(default)]
    pub session_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ReportArgs {
    /// Id of the step the evidence is for, as returned by get_current_step.
    pub step_id: String,
    /// Evidence matching the step's `submission_schema`.
    pub evidence: serde_json::Map<String, Value>,
    #[serde(default)]
    pub session_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CourseIdArgs {
    /// Course id (slug), as returned by authoring_list_courses.
    pub course_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CourseSourceArgs {
    /// The complete course definition as YAML (JSON is accepted too).
    pub source: String,
}

#[tool_router]
impl LearningPlatformServer {
    pub fn new(state: AppState) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Describe the learner's current course: objectives, sections and the status of every step."
    )]
    async fn get_course_context(
        &self,
        Parameters(args): Parameters<SessionArgs>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let snapshot = match self
            .begin("get_course_context", &parts, args.session_id.as_deref())
            .await
        {
            Ok(snapshot) => snapshot,
            Err(result) => return Ok(*result),
        };
        let course = &snapshot.course.course;
        let sections: Vec<Value> = course
            .sections
            .iter()
            .map(|section| {
                json!({
                    "id": section.id,
                    "title": section.title,
                    "steps": section.step_ids.iter().filter_map(|id| course.step(id)).map(|step| json!({
                        "id": step.id,
                        "title": step.title,
                        "type": step.step_type,
                        "status": snapshot.progress.status(&step.id),
                        "optional": step.optional,
                    })).collect::<Vec<_>>(),
                })
            })
            .collect();
        Ok(CallToolResult::structured(json!({
            "session_id": snapshot.session.id,
            "course": {
                "id": course.id,
                "version": course.version,
                "title": course.title,
                "summary": course.summary,
                "objectives": course.objectives,
            },
            "sections": sections,
        })))
    }

    #[tool(
        description = "Get the step the learner should work on now, with instructions and the evidence format expected by report_task_result."
    )]
    async fn get_current_step(
        &self,
        Parameters(args): Parameters<SessionArgs>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let snapshot = match self
            .begin("get_current_step", &parts, args.session_id.as_deref())
            .await
        {
            Ok(snapshot) => snapshot,
            Err(result) => return Ok(*result),
        };
        let course = &snapshot.course.course;
        let Some(step) = self.state.engine.current_step(course, &snapshot.progress) else {
            return Ok(CallToolResult::structured(json!({
                "session_id": snapshot.session.id,
                "course": course.title,
                "message": "Every available step is finished.",
                "session_status": snapshot.session.status,
            })));
        };
        let view = step_view(&self.state.engine, step);
        let state = snapshot.progress.state(&step.id);
        let how_to_complete = match view.completion {
            _ if view.client_task.is_some() => {
                "The learner runs this check from the step page in the web app: their browser sends the requests in client_task to the service they built, so it also works on localhost. Help them get the service working; you cannot submit this step."
            }
            lp_core::CompletionStrategy::Submission if view.submission_schema.is_some() => {
                "Help the learner complete the task, then call report_task_result with evidence matching submission_schema."
            }
            lp_core::CompletionStrategy::Event => {
                "This step completes automatically when the platform observes the required activity."
            }
            _ => "The learner completes this step in the web app.",
        };
        Ok(CallToolResult::structured(json!({
            "session_id": snapshot.session.id,
            "course": course.title,
            "step": view,
            "status": snapshot.progress.status(&step.id),
            "attempts": state.map_or(0, |state| state.attempts),
            "last_outcome": state.and_then(|state| state.last_outcome.clone()),
            "how_to_complete": how_to_complete,
        })))
    }

    #[tool(
        description = "Submit evidence for a practical step. The platform validates it and returns the checks that passed or failed."
    )]
    async fn report_task_result(
        &self,
        Parameters(args): Parameters<ReportArgs>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let snapshot = match self
            .begin("report_task_result", &parts, args.session_id.as_deref())
            .await
        {
            Ok(snapshot) => snapshot,
            Err(result) => return Ok(*result),
        };
        let principal = principal(&parts)?;
        let submission = Submission {
            source: SubmissionSource::Agent,
            payload: Value::Object(args.evidence),
        };
        let result = SessionService::new(&self.state)
            .submit(
                principal.user_id,
                snapshot.session.id,
                &args.step_id,
                submission,
            )
            .await;
        match result {
            Ok(result) => {
                let progress = ProgressDto::build(
                    &self.state.engine,
                    &result.snapshot.session,
                    &result.snapshot.course.course,
                    &result.snapshot.progress,
                );
                Ok(CallToolResult::structured(json!({
                    "step_id": args.step_id,
                    "outcome": result.outcome,
                    "step_status": result.snapshot.progress.status(&args.step_id),
                    "next_step_id": progress.current_step_id,
                    "course_progress_percent": progress.percent,
                })))
            }
            Err(error) => Ok(tool_error(&error)),
        }
    }

    #[tool(
        description = "The learner's project: the kind of website they build throughout the course track, their programming language, their interface language, and the seed data (catalog items and site texts) their site loads. Read it before helping with a practical step."
    )]
    async fn get_project(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let principal = principal(&parts)?;
        // Counts as activity in the learner's current course, if they have one.
        let _ = self.begin("get_project", &parts, None).await;
        let profile = match learners::profile(&self.state.db, principal.user_id).await {
            Ok(profile) => profile,
            Err(error) => return Ok(tool_error(&error.into())),
        };
        let Some(project) = profile.project else {
            return Ok(CallToolResult::structured(json!({
                "project": null,
                "message": "The learner has not chosen a project yet. They do that in the first course, \"Start your project\", in the web app.",
            })));
        };
        let theme = self.state.themes.get(&project.theme_id);
        Ok(CallToolResult::structured(json!({
            "project": {
                "theme": theme.map(|theme| theme.summary(profile.locale)),
                "stack": project.stack,
                "chosen_at": project.created_at,
            },
            "learner_language": profile.locale,
            "seed_data": theme.map(|theme| theme.data()),
            "note": "Explain things in the learner's language, but keep technical terms (embedding, endpoint, prompt, …) in English. Coach; let the learner write the code.",
        })))
    }

    #[tool(
        description = "Summarize progress of the learner's course session: percentage complete and status per step."
    )]
    async fn get_session_status(
        &self,
        Parameters(args): Parameters<SessionArgs>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let snapshot = match self
            .begin("get_session_status", &parts, args.session_id.as_deref())
            .await
        {
            Ok(snapshot) => snapshot,
            Err(result) => return Ok(*result),
        };
        let progress = ProgressDto::build(
            &self.state.engine,
            &snapshot.session,
            &snapshot.course.course,
            &snapshot.progress,
        );
        Ok(CallToolResult::structured(json!({
            "session_id": snapshot.session.id,
            "course": snapshot.course.course.title,
            "session_status": snapshot.session.status,
            "percent": progress.percent,
            "completed_required": progress.completed_required,
            "total_required": progress.total_required,
            "current_step_id": progress.current_step_id,
            "steps": progress.steps.iter().map(|step| json!({ "id": step.step_id, "status": step.status })).collect::<Vec<_>>(),
        })))
    }

    #[tool(
        description = "Super admin only. Course authoring guide: format rules, JSON schema of the course file and every available step type. Read this before writing a course."
    )]
    async fn authoring_get_guide(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        if let Err(result) = self.authoring(&parts).await {
            return Ok(*result);
        }
        Ok(CallToolResult::structured(authoring::guide(&self.state)))
    }

    #[tool(
        description = "Super admin only. List every course on the platform with its version, source (file or authored), whether it can be edited here and learner counts."
    )]
    async fn authoring_list_courses(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        if let Err(result) = self.authoring(&parts).await {
            return Ok(*result);
        }
        match authoring::list(&self.state).await {
            Ok(courses) => Ok(CallToolResult::structured(json!({ "courses": courses }))),
            Err(error) => Ok(tool_error(&error)),
        }
    }

    #[tool(
        description = "Super admin only. Get the YAML definition of a course's current version, including private validation rules."
    )]
    async fn authoring_get_course(
        &self,
        Parameters(args): Parameters<CourseIdArgs>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        if let Err(result) = self.authoring(&parts).await {
            return Ok(*result);
        }
        match authoring::get(&self.state, &args.course_id).await {
            Ok(course) => Ok(CallToolResult::structured(json!(course))),
            Err(error) => Ok(tool_error(&error)),
        }
    }

    #[tool(
        description = "Super admin only. Validate a course definition without saving it. Returns every problem with its location so you can fix them all at once."
    )]
    async fn authoring_validate_course(
        &self,
        Parameters(args): Parameters<CourseSourceArgs>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        if let Err(result) = self.authoring(&parts).await {
            return Ok(*result);
        }
        Ok(CallToolResult::structured(json!(authoring::validate(
            &self.state,
            &args.source
        ))))
    }

    #[tool(
        description = "Super admin only. Validate and publish a course definition. A new `id` creates a course; an existing authored course gets a new version, or is overwritten if no learner started that version yet. Learners see the result immediately."
    )]
    async fn authoring_publish_course(
        &self,
        Parameters(args): Parameters<CourseSourceArgs>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let principal = match self.authoring(&parts).await {
            Ok(principal) => principal,
            Err(result) => return Ok(*result),
        };
        match authoring::publish(&self.state, &args.source, principal.user_id).await {
            Ok(result) => Ok(CallToolResult::structured(json!({
                "status": result.status,
                "course": result.course,
                "url": format!("{}/courses/{}", self.state.config.public_base_url, result.course.id),
            }))),
            Err(ApiError::Unprocessable {
                message, details, ..
            }) => Ok(CallToolResult::structured_error(
                json!({ "error": message, "issues": details }),
            )),
            Err(error) => Ok(tool_error(&error)),
        }
    }
}

impl LearningPlatformServer {
    /// Authorizes an authoring tool call and records connection activity.
    async fn authoring(&self, parts: &Parts) -> Result<McpPrincipal, Box<CallToolResult>> {
        let principal =
            principal(parts).map_err(|error| tool_failure(error.message.into_owned()))?;
        if !principal.is_super_admin {
            return Err(tool_failure(
                "Course authoring requires a super admin account.".into(),
            ));
        }
        if let Err(error) =
            connections::record_activity(&self.state.db, principal.connection_id, None, None).await
        {
            tracing::warn!(%error, "could not record MCP activity");
        }
        Ok(principal)
    }
}

impl LearningPlatformServer {
    /// Resolves the session, reports the tool call to the engine and records
    /// connection activity. Returns the post-event snapshot so callers see any
    /// steps the call itself completed.
    async fn begin(
        &self,
        tool: &'static str,
        parts: &Parts,
        session_id: Option<&str>,
    ) -> Result<SessionSnapshot, Box<CallToolResult>> {
        let principal =
            principal(parts).map_err(|error| tool_failure(error.message.into_owned()))?;
        let session_id = session_id
            .map(Uuid::parse_str)
            .transpose()
            .map_err(|_| tool_failure("session_id must be a UUID".into()))?;
        let service = SessionService::new(&self.state);
        let snapshot = service
            .resolve_for_agent(principal.user_id, session_id)
            .await
            .map_err(|error| Box::new(tool_error(&error)))?;
        let client = parts
            .headers
            .get(http::header::USER_AGENT)
            .and_then(|value| value.to_str().ok())
            .map(|agent| agent.chars().take(120).collect::<String>());
        if let Err(error) = connections::record_activity(
            &self.state.db,
            principal.connection_id,
            client.as_deref(),
            Some(snapshot.session.id),
        )
        .await
        {
            tracing::warn!(%error, "could not record MCP activity");
        }
        let event = ExtensionEvent::new(TOOL_CALLED, json!({ "tool": tool, "client": client }));
        match service
            .dispatch_event(principal.user_id, snapshot.session.id, event)
            .await
        {
            Ok(result) => Ok(result.snapshot),
            // Completed or otherwise immutable sessions still answer read-only tools.
            Err(ApiError::Conflict { .. }) => Ok(snapshot),
            Err(error) => Err(Box::new(tool_error(&error))),
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for LearningPlatformServer {
    /// Authoring tools are only advertised to super admins.
    async fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::ListToolsResult, ErrorData> {
        let is_admin = context
            .extensions
            .get::<Parts>()
            .and_then(|parts| parts.extensions.get::<McpPrincipal>())
            .is_some_and(|principal| principal.is_super_admin);
        let tools = self
            .tool_router
            .list_all()
            .into_iter()
            .filter(|tool| is_admin || !tool.name.starts_with(AUTHORING_PREFIX))
            .collect();
        Ok(rmcp::model::ListToolsResult {
            result_type: Some(rmcp::model::ResultType::COMPLETE),
            tools,
            meta: None,
            next_cursor: None,
            ttl_ms: None,
            cache_scope: None,
        })
    }

    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(
                Implementation::new("learning-platform", env!("CARGO_PKG_VERSION"))
                    .with_title("Learning Platform"),
            )
            .with_instructions(INSTRUCTIONS)
    }
}

fn principal(parts: &Parts) -> Result<McpPrincipal, ErrorData> {
    parts
        .extensions
        .get::<McpPrincipal>()
        .cloned()
        .ok_or_else(|| ErrorData::invalid_request("unauthenticated", None))
}

fn tool_error(error: &ApiError) -> CallToolResult {
    let message = match error {
        ApiError::NotFound("active session") => {
            "The learner has no active course session. Ask them to start a course in the web app."
                .to_owned()
        }
        ApiError::Internal(_) => "The platform could not process this request.".to_owned(),
        ApiError::Unprocessable { message, .. } => format!("Evidence rejected: {message}"),
        other => other.to_string(),
    };
    CallToolResult::structured_error(json!({ "error": message }))
}

fn tool_failure(message: String) -> Box<CallToolResult> {
    Box::new(CallToolResult::structured_error(
        json!({ "error": message }),
    ))
}
