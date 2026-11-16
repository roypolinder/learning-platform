//! Compiled courses kept in memory. The current version of every course is loaded
//! at startup; older versions referenced by sessions are compiled on first use.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use lp_course_engine::{Course, CourseDefinition, CourseEngine};
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

#[derive(Default)]
pub struct CourseCatalog {
    current: RwLock<HashMap<String, Arc<LoadedCourse>>>,
    versions: RwLock<HashMap<Uuid, Arc<LoadedCourse>>>,
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

    /// Current versions as a learning path: by difficulty, then `order`, then title.
    pub fn list(&self) -> Vec<Arc<LoadedCourse>> {
        let mut courses: Vec<_> = read(&self.current).values().cloned().collect();
        courses.sort_by(|a, b| {
            let key = |c: &LoadedCourse| (c.course.difficulty, c.course.order.unwrap_or(u32::MAX));
            key(a)
                .cmp(&key(b))
                .then_with(|| a.course.title.cmp(&b.course.title))
        });
        courses
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
        let stored = repository::find_version(db, version_id)
            .await?
            .ok_or(ApiError::NotFound("course version"))?;
        let definition: CourseDefinition =
            serde_json::from_value(stored.definition).map_err(ApiError::internal)?;
        let course = engine.compile(definition).map_err(|error| {
            ApiError::Internal(format!(
                "stored course version {version_id} no longer compiles: {error}"
            ))
        })?;
        let loaded = Arc::new(LoadedCourse {
            course_uuid: stored.course_id,
            version_id,
            course,
        });
        write(&self.versions).insert(version_id, loaded.clone());
        Ok(loaded)
    }
}

fn read<T>(lock: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn write<T>(lock: &RwLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    lock.write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
