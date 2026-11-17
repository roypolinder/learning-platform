//! Publishes course files into the database. Each `(course, version)` pair is
//! immutable: changing content requires a version bump, so in-flight sessions
//! never see their steps change underneath them.

use std::path::Path;

use lp_course_engine::{
    CatalogError, CourseDefinition, CourseEngine, ThemeCatalog, check_renders, load_directory,
    load_themes,
};
use sqlx::PgPool;

use super::catalog::{CourseCatalog, LoadedCourse};
use super::repository;
use crate::security::sha256_hex;

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("course files are invalid:\n{0}")]
    InvalidCourses(String),
    #[error(
        "course `{course}` version {version} changed without a version bump; bump `version` or set MUTABLE_COURSE_VERSIONS=true for local authoring"
    )]
    ImmutableVersionChanged { course: String, version: String },
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("cannot serialize course definition: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Project themes live in `<courses>/themes/`.
pub fn load_theme_catalog(directory: &Path) -> Result<ThemeCatalog, SyncError> {
    load_themes(&directory.join("themes")).map_err(|errors| describe(&errors))
}

/// Human-readable report of course and theme problems, one issue per line.
pub fn describe(errors: &[CatalogError]) -> SyncError {
    SyncError::InvalidCourses(
        errors
            .iter()
            .map(|error| match error {
                CatalogError::Invalid { path, source } => format!(
                    "{}:\n{}",
                    path.display(),
                    source
                        .issues
                        .iter()
                        .map(|issue| format!("  - {issue}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                ),
                other => other.to_string(),
            })
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

pub async fn sync_courses(
    db: &PgPool,
    engine: &CourseEngine,
    themes: &ThemeCatalog,
    directory: &Path,
    allow_in_place_edits: bool,
) -> Result<CourseCatalog, SyncError> {
    let courses = load_directory(engine, directory).map_err(|errors| describe(&errors))?;
    let render_errors = check_renders(engine, &courses, themes);
    if !render_errors.is_empty() {
        return Err(describe(&render_errors));
    }

    let mut loaded = Vec::with_capacity(courses.len());
    let mut tx = db.begin().await?;
    for (_, course) in courses {
        let definition = serde_json::to_value(&course.definition)?;
        let checksum = sha256_hex(&serde_json::to_vec(&definition)?);
        let difficulty = serde_json::to_value(course.difficulty)?
            .as_str()
            .unwrap_or_default()
            .to_owned();
        let course_uuid = repository::upsert_course(
            &mut tx,
            &course.id,
            &course.title,
            &course.category,
            &difficulty,
        )
        .await?;

        let version_id = match repository::find_version_by_number(
            &mut tx,
            course_uuid,
            &course.version,
        )
        .await?
        {
            Some(existing) if existing.checksum == checksum => existing.id,
            Some(existing) if allow_in_place_edits => {
                repository::replace_version_definition(
                    &mut tx,
                    existing.id,
                    &checksum,
                    &definition,
                    None,
                )
                .await?;
                tracing::warn!(course = %course.id, version = %course.version, "course version edited in place");
                existing.id
            }
            Some(_) => {
                return Err(SyncError::ImmutableVersionChanged {
                    course: course.id.clone(),
                    version: course.version.clone(),
                });
            }
            None => {
                repository::insert_version(
                    &mut tx,
                    course_uuid,
                    &course.version,
                    &checksum,
                    &definition,
                    None,
                )
                .await?
            }
        };
        repository::set_current_version(&mut tx, course_uuid, version_id).await?;
        tracing::info!(course = %course.id, version = %course.version, steps = course.steps.len(), "course published");
        loaded.push(LoadedCourse {
            course_uuid,
            version_id,
            course,
        });
    }

    // Courses created in the app live only in the database.
    for authored in repository::authored_current_versions(&mut tx).await? {
        let compiled = serde_json::from_value::<CourseDefinition>(authored.definition)
            .map_err(|error| error.to_string())
            .and_then(|definition| {
                engine
                    .compile(definition)
                    .map_err(|error| error.to_string())
            });
        match compiled {
            Ok(course) => loaded.push(LoadedCourse {
                course_uuid: authored.course_id,
                version_id: authored.version_id,
                course,
            }),
            Err(error) => {
                tracing::warn!(course_id = %authored.course_id, %error, "authored course no longer compiles; not listed");
            }
        }
    }
    tx.commit().await?;
    Ok(CourseCatalog::new(loaded))
}
