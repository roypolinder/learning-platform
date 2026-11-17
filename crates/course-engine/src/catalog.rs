use std::path::{Path, PathBuf};

use crate::course::Course;
use crate::definition::CourseDefinition;
use crate::error::CourseValidationError;
use crate::localization::{CourseTranslation, Locale};
use crate::runtime::CourseEngine;
use crate::theme::ThemeCatalog;

/// Translation overlays live in `<courses>/locales/<lang>/<course id>.yaml`.
const LOCALES_DIR: &str = "locales";

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
    #[error("{path}: {message}")]
    Theme { path: PathBuf, message: String },
    #[error("{path}: translation for unknown course `{course}`")]
    OrphanTranslation { path: PathBuf, course: String },
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

/// `*.yaml` / `*.yml` files directly inside `directory`, sorted.
pub(crate) fn yaml_files(directory: &Path) -> Result<Vec<PathBuf>, Vec<CatalogError>> {
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
    Ok(paths)
}

/// Loads every course file directly inside `directory`, with its translations from
/// `locales/<lang>/`. All files are checked so authors see every problem at once.
pub fn load_directory(
    engine: &CourseEngine,
    directory: &Path,
) -> Result<Vec<(PathBuf, Course)>, Vec<CatalogError>> {
    let mut translations = load_translations(directory)?;
    let mut courses: Vec<(PathBuf, Course)> = Vec::new();
    let mut errors = Vec::new();
    for path in yaml_files(directory)? {
        match load_file(engine, &path, &mut translations) {
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
    errors.extend(
        translations
            .into_values()
            .flat_map(|by_locale| by_locale.into_values())
            .map(|(path, _)| {
                let course = file_stem(&path);
                CatalogError::OrphanTranslation { path, course }
            }),
    );
    if errors.is_empty() {
        Ok(courses)
    } else {
        Err(errors)
    }
}

/// Renders every course for every language and theme, so a placeholder a theme does
/// not define or a translation that breaks a step is caught before learners see it.
pub fn check_renders(
    engine: &CourseEngine,
    courses: &[(PathBuf, Course)],
    themes: &ThemeCatalog,
) -> Vec<CatalogError> {
    let mut errors = Vec::new();
    for (path, course) in courses {
        for theme in themes.selectable().iter().chain([themes.neutral()]) {
            for locale in Locale::ALL {
                let audience = crate::personalize::Audience {
                    locale,
                    theme: theme.as_ref(),
                };
                if let Err(source) = engine.render(course, audience) {
                    errors.push(CatalogError::Invalid {
                        path: path.clone(),
                        source,
                    });
                }
            }
        }
    }
    errors
}

type Translations = std::collections::BTreeMap<
    String,
    std::collections::BTreeMap<Locale, (PathBuf, CourseTranslation)>,
>;

fn load_translations(directory: &Path) -> Result<Translations, Vec<CatalogError>> {
    let mut translations = Translations::new();
    let mut errors = Vec::new();
    for locale in Locale::ALL
        .into_iter()
        .filter(|locale| *locale != Locale::SOURCE)
    {
        let locale_dir = directory.join(LOCALES_DIR).join(locale.as_str());
        if !locale_dir.is_dir() {
            continue;
        }
        for path in yaml_files(&locale_dir)? {
            let parsed = std::fs::read_to_string(&path)
                .map_err(|source| CatalogError::Io {
                    path: path.clone(),
                    source,
                })
                .and_then(|source| {
                    serde_saphyr::from_str::<CourseTranslation>(&source).map_err(|error| {
                        CatalogError::Parse {
                            path: path.clone(),
                            message: error.to_string(),
                        }
                    })
                });
            match parsed {
                Ok(translation) => {
                    translations
                        .entry(file_stem(&path))
                        .or_default()
                        .insert(locale, (path, translation));
                }
                Err(error) => errors.push(error),
            }
        }
    }
    if errors.is_empty() {
        Ok(translations)
    } else {
        Err(errors)
    }
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_owned()
}

fn load_file(
    engine: &CourseEngine,
    path: &Path,
    translations: &mut Translations,
) -> Result<Course, CatalogError> {
    let source = std::fs::read_to_string(path).map_err(|source| CatalogError::Io {
        path: path.to_owned(),
        source,
    })?;
    let mut definition = parse_definition(&source).map_err(|message| CatalogError::Parse {
        path: path.to_owned(),
        message,
    })?;
    for (locale, (file, translation)) in translations.remove(&definition.id).unwrap_or_default() {
        if definition
            .translations
            .insert(locale, translation)
            .is_some()
        {
            return Err(CatalogError::Parse {
                path: file,
                message: format!(
                    "`{}` already has an inline `{locale}` translation",
                    definition.id
                ),
            });
        }
    }
    engine
        .compile(definition)
        .map_err(|source| CatalogError::Invalid {
            path: path.to_owned(),
            source,
        })
}
