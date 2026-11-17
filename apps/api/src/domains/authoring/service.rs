//! Course authoring for super admins, shared by the HTTP API and the MCP gateway.
//! Authored courses are stored in the database only; file-managed courses stay
//! read-only here so the YAML in `COURSES_DIR` remains their source of truth.

use chrono::{DateTime, Utc};
use lp_course_engine::view::{CourseSummaryView, course_summary};
use lp_course_engine::{Audience, Locale};
use lp_course_engine::{Course, CourseIssue, parse_definition, render_definition};
use serde::Serialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domains::courses::catalog::LoadedCourse;
use crate::domains::courses::repository::{self, CourseRecord};
use crate::error::{ApiError, ApiResult};
use crate::security::sha256_hex;
use crate::state::AppState;

const MAX_SOURCE_BYTES: usize = 256 * 1024;
const AUTHORING_GUIDE: &str = include_str!("../../../../../docs/course-authoring.md");
const COURSE_SCHEMA: &str = include_str!("../../../../../courses/schema/course.schema.json");

#[derive(Debug, Serialize)]
pub struct AdminCourse {
    pub id: String,
    pub title: String,
    pub version: Option<String>,
    pub category: Option<String>,
    /// `file` (read-only, from COURSES_DIR) or `authored` (editable here).
    pub source: String,
    pub editable: bool,
    /// Whether the course is currently listed in the catalog.
    pub published: bool,
    pub step_count: usize,
    pub session_count: i64,
    pub completed_count: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct AdminCourseDetail {
    #[serde(flatten)]
    pub course: AdminCourse,
    /// YAML definition of the current version.
    pub definition: String,
}

#[derive(Debug, Serialize)]
pub struct ValidationReport {
    pub valid: bool,
    pub issues: Vec<CourseIssue>,
    pub course: Option<CourseSummaryView>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PublishStatus {
    Created,
    NewVersion,
    UpdatedInPlace,
    Unchanged,
}

#[derive(Debug, Serialize)]
pub struct PublishResult {
    pub status: PublishStatus,
    pub course: CourseSummaryView,
}

pub fn validate(state: &AppState, source: &str) -> ValidationReport {
    match compile(state, source) {
        Ok(course) => ValidationReport {
            valid: true,
            issues: Vec::new(),
            course: Some(course_summary(&course)),
        },
        Err(issues) => ValidationReport {
            valid: false,
            issues,
            course: None,
        },
    }
}

fn compile(state: &AppState, source: &str) -> Result<Course, Vec<CourseIssue>> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(vec![CourseIssue::new(
            "(document)",
            format!("course definition exceeds {} KiB", MAX_SOURCE_BYTES / 1024),
        )]);
    }
    let definition = parse_definition(source)
        .map_err(|message| vec![CourseIssue::new("(document)", message)])?;
    let course = state
        .engine
        .compile(definition)
        .map_err(|error| error.issues)?;
    // Every learner sees a rendered copy, so it must render for every language and theme.
    let mut issues = Vec::new();
    for theme in state
        .themes
        .selectable()
        .iter()
        .chain([state.themes.neutral()])
    {
        for locale in Locale::ALL {
            if let Err(error) = state.engine.render(
                &course,
                Audience {
                    locale,
                    theme: theme.as_ref(),
                },
            ) {
                issues.extend(error.issues);
            }
        }
    }
    issues.dedup();
    if issues.is_empty() {
        Ok(course)
    } else {
        Err(issues)
    }
}

pub async fn publish(state: &AppState, source: &str, author: Uuid) -> ApiResult<PublishResult> {
    let course = compile(state, source).map_err(|issues| ApiError::Unprocessable {
        code: "invalid_course",
        message: format!("The course definition has {} problem(s).", issues.len()),
        details: serde_json::to_value(&issues).ok(),
    })?;
    let definition = serde_json::to_value(&course.definition).map_err(ApiError::internal)?;
    let checksum = sha256_hex(&serde_json::to_vec(&definition).map_err(ApiError::internal)?);
    let difficulty = serde_json::to_value(course.difficulty)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default();

    let mut tx = state.db.begin().await?;
    let course_uuid = repository::upsert_authored_course(
        &mut tx,
        &course.id,
        &course.title,
        &course.category,
        &difficulty,
    )
    .await?
    .ok_or_else(|| ApiError::Conflict {
        code: "course_managed_by_files",
        message: format!(
            "Course `{}` is defined by a file in the courses directory. Edit that file, or choose a different `id`.",
            course.id
        ),
    })?;

    let existing =
        repository::find_version_by_number(&mut tx, course_uuid, &course.version).await?;
    let (status, version_id) = match existing {
        None => {
            let created = repository::find_record(&mut *tx, &course.id)
                .await?
                .is_none_or(|record| record.version.is_none());
            let id = repository::insert_version(
                &mut tx,
                course_uuid,
                &course.version,
                &checksum,
                &definition,
                Some((source, author)),
            )
            .await?;
            let status = if created {
                PublishStatus::Created
            } else {
                PublishStatus::NewVersion
            };
            (status, id)
        }
        Some(existing) if existing.checksum == checksum => {
            // Keep the latest formatting even when the content is identical.
            repository::replace_version_definition(
                &mut tx,
                existing.id,
                &checksum,
                &definition,
                Some(source),
            )
            .await?;
            (PublishStatus::Unchanged, existing.id)
        }
        Some(existing) => {
            let in_use = repository::count_sessions_for_version(&mut tx, existing.id).await? > 0;
            if in_use && !state.config.mutable_course_versions {
                return Err(ApiError::Conflict {
                    code: "version_in_use",
                    message: format!(
                        "Learners already started version {} of this course. Bump `version` to publish changes.",
                        course.version
                    ),
                });
            }
            repository::replace_version_definition(
                &mut tx,
                existing.id,
                &checksum,
                &definition,
                Some(source),
            )
            .await?;
            (PublishStatus::UpdatedInPlace, existing.id)
        }
    };
    repository::set_current_version(&mut tx, course_uuid, version_id).await?;
    tx.commit().await?;

    tracing::info!(course = %course.id, version = %course.version, ?status, %author, "course published from authoring");
    let summary = course_summary(&course);
    state.catalog.publish(LoadedCourse {
        course_uuid,
        version_id,
        course,
    });
    Ok(PublishResult {
        status,
        course: summary,
    })
}

pub async fn list(state: &AppState) -> ApiResult<Vec<AdminCourse>> {
    let records = repository::list_records(&state.db).await?;
    Ok(records
        .iter()
        .map(|record| admin_course(state, record))
        .collect())
}

pub async fn get(state: &AppState, slug: &str) -> ApiResult<AdminCourseDetail> {
    let record = repository::find_record(&state.db, slug)
        .await?
        .ok_or(ApiError::NotFound("course"))?;
    let definition = match (&record.source_text, &record.definition) {
        (Some(source), _) => source.clone(),
        (None, Some(definition)) => render_definition(definition).map_err(ApiError::internal)?,
        (None, None) => return Err(ApiError::NotFound("course version")),
    };
    Ok(AdminCourseDetail {
        course: admin_course(state, &record),
        definition,
    })
}

fn admin_course(state: &AppState, record: &CourseRecord) -> AdminCourse {
    let field = |name: &str| {
        record
            .definition
            .as_ref()
            .and_then(|definition| definition.get(name))
            .and_then(Value::as_str)
            .map(str::to_owned)
    };
    let loaded = state.catalog.current(&record.slug);
    AdminCourse {
        id: record.slug.clone(),
        title: field("title").unwrap_or_else(|| record.slug.clone()),
        version: record.version.clone(),
        category: field("category"),
        editable: record.source == "authored",
        source: record.source.clone(),
        published: loaded.is_some(),
        step_count: loaded.map_or(0, |loaded| loaded.course.steps.len()),
        session_count: record.session_count,
        completed_count: record.completed_count,
        updated_at: record.updated_at,
    }
}

/// Everything an author (human or agent) needs to write a valid course.
pub fn guide(state: &AppState) -> Value {
    let step_types: Vec<Value> = state
        .engine
        .step_types()
        .map(|entry| {
            json!({
                "name": entry.spec.name,
                "category": entry.spec.category,
                "completion": entry.spec.completion,
                "description": entry.spec.description,
                "provider": entry.provider,
            })
        })
        .collect();
    json!({
        "format": "YAML (JSON is accepted too). One document per course.",
        "rules": [
            "`id` is the course slug: stable, lowercase, never change it after publishing.",
            "Bump `version` whenever learners may already have started the course; unused versions can be overwritten.",
            "`validation` blocks are private and never shown to learners.",
            "Courses whose `id` matches a file in the courses directory cannot be edited here."
        ],
        "guide_markdown": AUTHORING_GUIDE,
        "json_schema": serde_json::from_str::<Value>(COURSE_SCHEMA).unwrap_or(Value::Null),
        "step_types": step_types,
    })
}
