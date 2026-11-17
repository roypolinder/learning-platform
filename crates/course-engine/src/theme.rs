//! Project themes: the kind of website a learner builds throughout the course track.
//!
//! Courses are written once and refer to the learner's theme through `{{theme.…}}`
//! placeholders (see [`crate::personalize`]). A theme file supplies:
//!
//! - `vars`: localized wording such as `item: bike`, shared by every theme,
//! - `data`: the seed data learners load into their own site (public),
//! - `probes`: hidden test inputs and expectations that fit that data (private).
//!
//! The non-selectable `neutral` theme defines the wording used before a learner has
//! chosen a project. Every selectable theme must define the same `vars`, and `data` and
//! `probes` with the same shape, so placeholders resolve to the same kind of value for
//! every learner.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::catalog::CatalogError;
use crate::localization::Locale;

pub const NEUTRAL_THEME: &str = "neutral";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeDefinition {
    pub id: String,
    /// Position in the theme picker; lower comes first.
    #[serde(default)]
    pub order: u32,
    /// `false` only for the neutral theme.
    #[serde(default = "selectable_by_default")]
    pub selectable: bool,
    #[serde(default)]
    pub icon: Option<String>,
    pub name: BTreeMap<Locale, String>,
    #[serde(default)]
    pub tagline: BTreeMap<Locale, String>,
    pub vars: BTreeMap<Locale, BTreeMap<String, String>>,
    /// Seed data learners load into their project. Public.
    #[serde(default)]
    pub data: Value,
    /// Hidden test inputs and expectations matching `data`. Never sent to learners.
    #[serde(default)]
    pub probes: Value,
}

fn selectable_by_default() -> bool {
    true
}

#[derive(Debug)]
pub struct Theme {
    pub definition: ThemeDefinition,
    scopes: HashMap<Locale, Value>,
}

/// Learner-facing description of a theme in one language.
#[derive(Debug, Clone, Serialize)]
pub struct ThemeSummary {
    pub id: String,
    pub name: String,
    pub tagline: String,
    pub icon: Option<String>,
}

impl Theme {
    fn new(definition: ThemeDefinition) -> Self {
        let scopes = Locale::ALL
            .iter()
            .map(|&locale| {
                let mut scope = Map::new();
                scope.insert("id".into(), definition.id.clone().into());
                scope.insert("name".into(), localized(&definition.name, locale).into());
                scope.insert(
                    "tagline".into(),
                    localized(&definition.tagline, locale).into(),
                );
                if let Some(vars) = definition.vars.get(&locale) {
                    for (key, value) in vars {
                        scope.insert(key.clone(), value.clone().into());
                    }
                }
                scope.insert("data".into(), definition.data.clone());
                scope.insert("probes".into(), definition.probes.clone());
                (locale, Value::Object(scope))
            })
            .collect();
        Self { definition, scopes }
    }

    pub fn id(&self) -> &str {
        &self.definition.id
    }

    pub fn is_selectable(&self) -> bool {
        self.definition.selectable
    }

    /// Values available to `{{theme.…}}` placeholders in `locale`.
    pub fn scope(&self, locale: Locale) -> &Value {
        &self.scopes[&locale]
    }

    pub fn summary(&self, locale: Locale) -> ThemeSummary {
        ThemeSummary {
            id: self.definition.id.clone(),
            name: localized(&self.definition.name, locale),
            tagline: localized(&self.definition.tagline, locale),
            icon: self.definition.icon.clone(),
        }
    }

    pub fn data(&self) -> &Value {
        &self.definition.data
    }
}

fn localized(values: &BTreeMap<Locale, String>, locale: Locale) -> String {
    values
        .get(&locale)
        .or_else(|| values.get(&Locale::SOURCE))
        .cloned()
        .unwrap_or_default()
}

#[derive(Debug)]
pub struct ThemeCatalog {
    themes: Vec<Arc<Theme>>,
    neutral: Arc<Theme>,
}

impl Default for ThemeCatalog {
    fn default() -> Self {
        Self {
            themes: Vec::new(),
            neutral: Arc::new(Theme::new(ThemeDefinition {
                id: NEUTRAL_THEME.into(),
                order: 0,
                selectable: false,
                icon: None,
                name: BTreeMap::new(),
                tagline: BTreeMap::new(),
                vars: BTreeMap::new(),
                data: Value::Null,
                probes: Value::Null,
            })),
        }
    }
}

impl ThemeCatalog {
    pub fn from_definitions(
        definitions: Vec<ThemeDefinition>,
    ) -> Result<Self, Vec<(String, String)>> {
        let mut problems = Vec::new();
        let mut catalog = Self::default();
        let mut seen = BTreeSet::new();
        let mut neutral = None;
        for definition in definitions {
            let id = definition.id.clone();
            if !seen.insert(id.clone()) {
                problems.push((id.clone(), "duplicate theme id".into()));
            }
            if !id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            {
                problems.push((
                    id.clone(),
                    "id must be lowercase letters, digits and '-'".into(),
                ));
            }
            for locale in Locale::ALL {
                if definition.selectable && !definition.name.contains_key(&locale) {
                    problems.push((id.clone(), format!("name.{locale} is missing")));
                }
                if !definition.vars.contains_key(&locale) {
                    problems.push((id.clone(), format!("vars.{locale} is missing")));
                }
            }
            match (definition.selectable, id == NEUTRAL_THEME) {
                (false, true) => neutral = Some(definition),
                (true, false) => catalog.themes.push(Arc::new(Theme::new(definition))),
                _ => problems.push((
                    id,
                    format!("only `{NEUTRAL_THEME}` is not selectable, and it must not be"),
                )),
            }
        }
        match neutral {
            Some(definition) => catalog.neutral = Arc::new(Theme::new(definition)),
            None if catalog.themes.is_empty() => {}
            None => problems.push((NEUTRAL_THEME.into(), "the neutral theme is missing".into())),
        }
        for theme in &catalog.themes {
            for locale in Locale::ALL {
                let expected: BTreeSet<&String> = catalog
                    .neutral
                    .definition
                    .vars
                    .get(&locale)
                    .map(|vars| vars.keys().collect())
                    .unwrap_or_default();
                let actual: BTreeSet<&String> = theme
                    .definition
                    .vars
                    .get(&locale)
                    .map(|vars| vars.keys().collect())
                    .unwrap_or_default();
                for key in expected.difference(&actual) {
                    problems.push((theme.id().into(), format!("vars.{locale}.{key} is missing")));
                }
                for key in actual.difference(&expected) {
                    problems.push((
                        theme.id().into(),
                        format!("vars.{locale}.{key} is not defined by the neutral theme"),
                    ));
                }
            }
        }
        for theme in &catalog.themes {
            for (section, canonical, actual) in [
                (
                    "data",
                    &catalog.neutral.definition.data,
                    &theme.definition.data,
                ),
                (
                    "probes",
                    &catalog.neutral.definition.probes,
                    &theme.definition.probes,
                ),
            ] {
                let mut differences = Vec::new();
                compare_shape(canonical, actual, section, &mut differences);
                problems.extend(
                    differences
                        .into_iter()
                        .map(|difference| (theme.id().to_owned(), difference)),
                );
            }
        }
        catalog
            .themes
            .sort_by(|a, b| (a.definition.order, a.id()).cmp(&(b.definition.order, b.id())));
        if problems.is_empty() {
            Ok(catalog)
        } else {
            Err(problems)
        }
    }

    /// Themes a learner can choose, in picker order.
    pub fn selectable(&self) -> &[Arc<Theme>] {
        &self.themes
    }

    pub fn get(&self, id: &str) -> Option<&Arc<Theme>> {
        self.themes.iter().find(|theme| theme.id() == id)
    }

    /// Wording used before a learner has chosen a project.
    pub fn neutral(&self) -> &Arc<Theme> {
        &self.neutral
    }

    /// The learner's theme, or the neutral one when they have none (or it was removed).
    pub fn resolve(&self, id: Option<&str>) -> &Arc<Theme> {
        id.and_then(|id| self.get(id)).unwrap_or(&self.neutral)
    }
}

/// The neutral theme defines the shape of `data` and `probes`: every theme needs the same
/// keys with the same kinds of values, so a course can rely on them for any learner.
fn compare_shape(canonical: &Value, actual: &Value, path: &str, differences: &mut Vec<String>) {
    match (canonical, actual) {
        (Value::Object(expected), Value::Object(found)) => {
            for (key, value) in expected {
                let child = format!("{path}.{key}");
                match found.get(key) {
                    Some(other) => compare_shape(value, other, &child, differences),
                    None => differences.push(format!("{child} is missing")),
                }
            }
            for key in found.keys().filter(|key| !expected.contains_key(*key)) {
                differences.push(format!("{path}.{key} is not defined by the neutral theme"));
            }
        }
        (Value::Array(expected), Value::Array(found)) => {
            if let Some(template) = expected.first() {
                for (index, item) in found.iter().enumerate() {
                    compare_shape(template, item, &format!("{path}[{index}]"), differences);
                }
            }
        }
        (Value::Null, _) => {}
        (expected, found) if kind(expected) == kind(found) => {}
        (expected, found) => differences.push(format!(
            "{path} must be {} like in the neutral theme, not {}",
            kind(expected),
            kind(found)
        )),
    }
}

fn kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "text",
        Value::Array(_) => "a list",
        Value::Object(_) => "an object",
    }
}

/// Loads every theme in `directory`. A missing directory means no themes.
pub fn load_themes(directory: &Path) -> Result<ThemeCatalog, Vec<CatalogError>> {
    if !directory.is_dir() {
        return Ok(ThemeCatalog::default());
    }
    let mut errors = Vec::new();
    let mut definitions = Vec::new();
    let mut paths: HashMap<String, PathBuf> = HashMap::new();
    for path in crate::catalog::yaml_files(directory)? {
        match std::fs::read_to_string(&path) {
            Ok(source) => match serde_saphyr::from_str::<ThemeDefinition>(&source) {
                Ok(definition) => {
                    paths.insert(definition.id.clone(), path);
                    definitions.push(definition);
                }
                Err(error) => errors.push(CatalogError::Parse {
                    path,
                    message: error.to_string(),
                }),
            },
            Err(source) => errors.push(CatalogError::Io { path, source }),
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    ThemeCatalog::from_definitions(definitions).map_err(|problems| {
        problems
            .into_iter()
            .map(|(id, message)| CatalogError::Theme {
                path: paths
                    .get(&id)
                    .cloned()
                    .unwrap_or_else(|| directory.to_owned()),
                message,
            })
            .collect()
    })
}
