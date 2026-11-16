use std::path::{Path, PathBuf};

use crate::course::Course;
use crate::definition::CourseDefinition;
use crate::error::CourseValidationError;
use crate::runtime::CourseEngine;

#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    #[error("cannot read {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("cannot parse {path}: {message}")]
    Parse { path: PathBuf, message: String },
    #[error("{path}: {source}")]
    Invalid {
        path: PathBuf,
        source: CourseValidationError,
    },
    #[error("course id `{id}` is defined in both {first} and {second}")]
    DuplicateCourse {
        id: String,
        first: PathBuf,
        second: PathBuf,
    },
}

pub fn parse_definition(source: &str) -> Result<CourseDefinition, String> {
    serde_saphyr::from_str(source).map_err(|error| error.to_string())
}

/// Renders a stored definition back to authoring YAML, omitting unset fields.
pub fn render_definition(definition: &serde_json::Value) -> Result<String, String> {
    fn prune(value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(map) => map
                .iter()
                .filter(|(_, v)| !v.is_null())
                .map(|(k, v)| (k.clone(), prune(v)))
                .collect(),
            serde_json::Value::Array(items) => items.iter().map(prune).collect(),
            other => other.clone(),
        }
    }
    serde_saphyr::to_string(&prune(definition)).map_err(|error| error.to_string())
}

/// Loads every `*.yaml` / `*.yml` file directly inside `directory`. All files are
/// checked so authors see every problem at once.
pub fn load_directory(
    engine: &CourseEngine,
    directory: &Path,
) -> Result<Vec<(PathBuf, Course)>, Vec<CatalogError>> {
    let entries = std::fs::read_dir(directory).map_err(|source| {
        vec![CatalogError::Io {
            path: directory.to_owned(),
            source,
        }]
    })?;
    let mut paths: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && matches!(
                    path.extension().and_then(|ext| ext.to_str()),
                    Some("yaml" | "yml")
                )
        })
        .collect();
    paths.sort();

    let mut courses: Vec<(PathBuf, Course)> = Vec::new();
    let mut errors = Vec::new();
    for path in paths {
        match load_file(engine, &path) {
            Ok(course) => {
                if let Some((first, _)) = courses.iter().find(|(_, c)| c.id == course.id) {
                    errors.push(CatalogError::DuplicateCourse {
                        id: course.id.clone(),
                        first: first.clone(),
                        second: path,
                    });
                } else {
                    courses.push((path, course));
                }
            }
            Err(error) => errors.push(error),
        }
    }
    if errors.is_empty() {
        Ok(courses)
    } else {
        Err(errors)
    }
}

fn load_file(engine: &CourseEngine, path: &Path) -> Result<Course, CatalogError> {
    let source = std::fs::read_to_string(path).map_err(|source| CatalogError::Io {
        path: path.to_owned(),
        source,
    })?;
    let definition = parse_definition(&source).map_err(|message| CatalogError::Parse {
        path: path.to_owned(),
        message,
    })?;
    engine
        .compile(definition)
        .map_err(|source| CatalogError::Invalid {
            path: path.to_owned(),
            source,
        })
}
