//! Compiled courses kept in memory. The current version of every course is loaded
//! at startup; older versions referenced by sessions are compiled on first use.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use lp_course_engine::{Audience, Course, CourseDefinition, CourseEngine, Locale};
use sqlx::PgPool;
use uuid::Uuid;

use super::repository;
use crate::error::{ApiError, ApiResult};

#[derive(Debug)]
pub struct LoadedCourse {
    pub course_uuid: Uuid,
    pub version_id: Uuid,
    pub course: Course,
}

/// A course version rendered for one language and project theme.
type RenderKey = (Uuid, Locale, String);

#[derive(Default)]
pub struct CourseCatalog {
    current: RwLock<HashMap<String, Arc<LoadedCourse>>>,
    versions: RwLock<HashMap<Uuid, Arc<LoadedCourse>>>,
    rendered: RwLock<HashMap<RenderKey, Arc<LoadedCourse>>>,
    /// Stored versions that no longer compile (e.g. after a step type was removed).
    /// Remembered so they are reported once instead of on every request.
    retired: RwLock<HashSet<Uuid>>,
}

impl CourseCatalog {
    pub fn new(courses: Vec<LoadedCourse>) -> Self {
        let catalog = Self::default();
        for course in courses {
            let course = Arc::new(course);
            write(&catalog.versions).insert(course.version_id, course.clone());
            write(&catalog.current).insert(course.course.id.clone(), course);
        }
        catalog
    }

    pub fn current(&self, slug: &str) -> Option<Arc<LoadedCourse>> {
        read(&self.current).get(slug).cloned()
    }

    /// Makes `course` the current version of its slug.
    pub fn publish(&self, course: LoadedCourse) {
        let course = Arc::new(course);
        write(&self.versions).insert(course.version_id, course.clone());
        write(&self.current).insert(course.course.id.clone(), course);
    }

    /// Current versions as one learning track: by `order`, then difficulty, then title.
    pub fn list(&self) -> Vec<Arc<LoadedCourse>> {
        let mut courses: Vec<_> = read(&self.current).values().cloned().collect();
        courses.sort_by(|a, b| {
            let key = |c: &LoadedCourse| (c.course.order.unwrap_or(u32::MAX), c.course.difficulty);
            key(a)
                .cmp(&key(b))
                .then_with(|| a.course.title.cmp(&b.course.title))
        });
        courses
    }

    /// `loaded` as a learner with `audience` sees it. Cached: there are only a few
    /// languages and themes per version.
    pub fn render(
        &self,
        engine: &CourseEngine,
        loaded: &LoadedCourse,
        audience: Audience<'_>,
    ) -> ApiResult<Arc<LoadedCourse>> {
        let key = (
            loaded.version_id,
            audience.locale,
            audience.theme.id().to_owned(),
        );
        if let Some(course) = read(&self.rendered).get(&key).cloned() {
            return Ok(course);
        }
        let course = engine.render(&loaded.course, audience).map_err(|error| {
            ApiError::Internal(format!(
                "course {} cannot be rendered for {} / {}: {error}",
                loaded.course.id, key.1, key.2
            ))
        })?;
        let rendered = Arc::new(LoadedCourse {
            course_uuid: loaded.course_uuid,
            version_id: loaded.version_id,
            course,
        });
        write(&self.rendered).insert(key, rendered.clone());
        Ok(rendered)
    }

    pub async fn version(
        &self,
        db: &PgPool,
        engine: &CourseEngine,
        version_id: Uuid,
    ) -> ApiResult<Arc<LoadedCourse>> {
        if let Some(course) = read(&self.versions).get(&version_id).cloned() {
            return Ok(course);
        }
        if read(&self.retired).contains(&version_id) {
            return Err(retired());
        }
        let stored = repository::find_version(db, version_id)
            .await?
            .ok_or(ApiError::NotFound("course version"))?;
        let definition: CourseDefinition =
            serde_json::from_value(stored.definition).map_err(ApiError::internal)?;
        let course = match engine.compile(definition) {
            Ok(course) => course,
            Err(error) => {
                tracing::warn!(%version_id, %error, "stored course version no longer compiles; its sessions are retired");
                write(&self.retired).insert(version_id);
                return Err(retired());
            }
        };
        let loaded = Arc::new(LoadedCourse {
            course_uuid: stored.course_id,
            version_id,
            course,
        });
        write(&self.versions).insert(version_id, loaded.clone());
        Ok(loaded)
    }
}

pub const RETIRED_VERSION: &str = "course_version_retired";

fn retired() -> ApiError {
    ApiError::Conflict {
        code: RETIRED_VERSION,
        message: "This session belongs to an old version of the course that is no longer available. Start the course again.".into(),
    }
}

fn read<T>(lock: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn write<T>(lock: &RwLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    lock.write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
