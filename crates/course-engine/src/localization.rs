//! Course translations. A course is authored in English; other languages are overlays
//! keyed by section and step id that may only replace text, never structure or rules.

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::definition::{CourseDefinition, SharedSectionDefinition, StepDefinition};
use crate::error::CourseIssue;

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    #[default]
    En,
    Nl,
}

impl Locale {
    pub const ALL: [Locale; 2] = [Locale::En, Locale::Nl];
    /// The language courses are authored in.
    pub const SOURCE: Locale = Locale::En;

    pub fn as_str(self) -> &'static str {
        match self {
            Locale::En => "en",
            Locale::Nl => "nl",
        }
    }

    /// Picks the best supported language from an `Accept-Language` header.
    pub fn negotiate(accept_language: &str) -> Locale {
        let mut ranked: Vec<(f32, Locale)> = accept_language
            .split(',')
            .filter_map(|part| {
                let mut pieces = part.trim().split(';');
                let tag = pieces.next()?.trim();
                let quality = pieces
                    .find_map(|piece| piece.trim().strip_prefix("q="))
                    .and_then(|q| q.parse::<f32>().ok())
                    .unwrap_or(1.0);
                let primary = tag.split(['-', '_']).next()?;
                primary
                    .parse::<Locale>()
                    .ok()
                    .map(|locale| (quality, locale))
            })
            .collect();
        ranked.sort_by(|a, b| b.0.total_cmp(&a.0));
        ranked
            .first()
            .map(|(_, locale)| *locale)
            .unwrap_or_default()
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Locale {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "en" => Ok(Locale::En),
            "nl" => Ok(Locale::Nl),
            other => Err(format!("unsupported language `{other}`")),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CourseTranslation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    /// Same length and order as the source objectives.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub objectives: Option<Vec<String>>,
    /// Prerequisite titles, same length and order as the source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prerequisites: Option<Vec<String>>,
    /// Term → translated definition. Terms themselves stay in English.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub glossary: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub sections: BTreeMap<String, SectionTranslation>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub shared_sections: BTreeMap<String, SectionTranslation>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SectionTranslation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub steps: BTreeMap<String, StepTranslation>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepTranslation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    /// Text inside `config`, mirroring its shape (see [`merge_text`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<Value>,
    /// Learner-visible text inside `validation`, such as check labels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation: Option<Value>,
}

/// Returns the definition in `locale`, without translations. Problems are reported with
/// paths under `translations.<locale>`.
pub fn localize(
    definition: &CourseDefinition,
    locale: Locale,
) -> Result<CourseDefinition, Vec<CourseIssue>> {
    let mut localized = definition.clone();
    localized.translations.clear();
    if locale == Locale::SOURCE {
        return Ok(localized);
    }
    let Some(translation) = definition.translations.get(&locale) else {
        return Ok(localized);
    };
    let root = format!("translations.{locale}");
    let mut issues = Vec::new();

    replace(&mut localized.title, &translation.title);
    replace(&mut localized.summary, &translation.summary);
    replace(&mut localized.description, &translation.description);
    replace(&mut localized.category, &translation.category);
    if let Some(help) = &translation.help {
        localized.help = Some(help.clone());
    }
    if let Some(objectives) = &translation.objectives {
        if objectives.len() == localized.objectives.len() {
            localized.objectives = objectives.clone();
        } else {
            issues.push(CourseIssue::new(
                format!("{root}.objectives"),
                format!("needs {} entries", localized.objectives.len()),
            ));
        }
    }
    if let Some(titles) = &translation.prerequisites {
        if titles.len() == localized.prerequisites.len() {
            for (prerequisite, title) in localized.prerequisites.iter_mut().zip(titles) {
                prerequisite.title = title.clone();
            }
        } else {
            issues.push(CourseIssue::new(
                format!("{root}.prerequisites"),
                format!("needs {} entries", localized.prerequisites.len()),
            ));
        }
    }
    for (term, text) in &translation.glossary {
        match localized
            .glossary
            .iter_mut()
            .find(|entry| &entry.term == term)
        {
            Some(entry) => entry.definition = text.clone(),
            None => issues.push(CourseIssue::new(
                format!("{root}.glossary.{term}"),
                "unknown glossary term",
            )),
        }
    }
    for (id, section_translation) in &translation.sections {
        let path = format!("{root}.sections.{id}");
        match localized.sections.iter_mut().find(|s| &s.id == id) {
            Some(section) => {
                replace(&mut section.title, &section_translation.title);
                if let Some(description) = &section_translation.description {
                    section.description = Some(description.clone());
                }
                translate_steps(
                    &mut section.steps,
                    &section_translation.steps,
                    &path,
                    &mut issues,
                );
            }
            None => issues.push(CourseIssue::new(path, "unknown section")),
        }
    }
    for (id, section_translation) in &translation.shared_sections {
        let path = format!("{root}.shared_sections.{id}");
        match localized.shared_sections.get_mut(id) {
            Some(SharedSectionDefinition { description, steps }) => {
                if section_translation.title.is_some() {
                    issues.push(CourseIssue::new(
                        format!("{path}.title"),
                        "shared sections have no title",
                    ));
                }
                if let Some(text) = &section_translation.description {
                    *description = Some(text.clone());
                }
                translate_steps(steps, &section_translation.steps, &path, &mut issues);
            }
            None => issues.push(CourseIssue::new(path, "unknown shared section")),
        }
    }

    if issues.is_empty() {
        Ok(localized)
    } else {
        Err(issues)
    }
}

fn translate_steps(
    steps: &mut [StepDefinition],
    translations: &BTreeMap<String, StepTranslation>,
    section_path: &str,
    issues: &mut Vec<CourseIssue>,
) {
    for (id, translation) in translations {
        let path = format!("{section_path}.steps.{id}");
        let Some(step) = steps.iter_mut().find(|step| &step.id == id) else {
            issues.push(CourseIssue::new(path, "unknown step"));
            continue;
        };
        replace(&mut step.title, &translation.title);
        for (target, text) in [
            (&mut step.description, &translation.description),
            (&mut step.body, &translation.body),
            (&mut step.help, &translation.help),
        ] {
            if let Some(text) = text {
                *target = Some(text.clone());
            }
        }
        for (name, target, overlay) in [
            ("config", &mut step.config, &translation.config),
            ("validation", &mut step.validation, &translation.validation),
        ] {
            if let Some(overlay) = overlay {
                let mut messages = Vec::new();
                merge_text(target, overlay, name, &mut messages);
                issues.extend(
                    messages
                        .into_iter()
                        .map(|message| CourseIssue::new(path.clone(), message)),
                );
            }
        }
    }
}

fn replace(target: &mut String, translation: &Option<String>) {
    if let Some(text) = translation {
        *target = text.clone();
    }
}

/// Overlays translated text onto `base`. Strings replace strings; objects merge key by
/// key; arrays of objects match items by `id` (or by position when items have none);
/// arrays of plain strings, such as keyword lists, are replaced as a whole. Anything
/// else would change behaviour rather than wording and is rejected.
pub fn merge_text(base: &mut Value, overlay: &Value, path: &str, issues: &mut Vec<String>) {
    match (base, overlay) {
        (Value::String(target), Value::String(text)) => *target = text.clone(),
        (Value::Object(target), Value::Object(fields)) => {
            for (key, value) in fields {
                let field_path = format!("{path}.{key}");
                match target.get_mut(key) {
                    Some(existing) => merge_text(existing, value, &field_path, issues),
                    None => issues.push(format!("{field_path}: not present in the source")),
                }
            }
        }
        (Value::Array(target), Value::Array(items)) if items.iter().all(Value::is_string) => {
            if target.iter().all(Value::is_string) {
                *target = items.clone();
            } else {
                issues.push(format!("{path}: only lists of plain text can be replaced"));
            }
        }
        (Value::Array(target), Value::Array(items)) => {
            for (index, item) in items.iter().enumerate() {
                let item_path = format!("{path}[{index}]");
                let existing = match item.get("id").and_then(Value::as_str) {
                    Some(id) => target
                        .iter_mut()
                        .find(|candidate| candidate.get("id").and_then(Value::as_str) == Some(id)),
                    None => target.get_mut(index),
                };
                match existing {
                    Some(existing) => merge_text(existing, item, &item_path, issues),
                    None => issues.push(format!("{item_path}: no matching item in the source")),
                }
            }
        }
        (_, _) => issues.push(format!("{path}: translations may only change text")),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn negotiates_accept_language() {
        assert_eq!(Locale::negotiate("nl-NL,nl;q=0.9,en;q=0.8"), Locale::Nl);
        assert_eq!(Locale::negotiate("de-DE,en;q=0.5,nl;q=0.7"), Locale::Nl);
        assert_eq!(Locale::negotiate("fr"), Locale::En);
        assert_eq!(Locale::negotiate(""), Locale::En);
    }

    #[test]
    fn merges_text_by_id_and_rejects_rule_changes() {
        let mut config = json!({
            "prompt": "Pick one",
            "options": [ { "id": "a", "text": "Yes", "correct": true }, { "id": "b", "text": "No" } ],
            "keywords": ["hybrid", "both"]
        });
        let mut issues = Vec::new();
        merge_text(
            &mut config,
            &json!({
                "prompt": "Kies er één",
                "options": [ { "id": "b", "text": "Nee" } ],
                "keywords": ["hybrid", "beide", "both"]
            }),
            "config",
            &mut issues,
        );
        assert!(issues.is_empty(), "{issues:?}");
        assert_eq!(config["options"][1]["text"], "Nee");
        assert_eq!(config["options"][0]["text"], "Yes");
        assert_eq!(config["keywords"][1], "beide");

        merge_text(
            &mut config,
            &json!({ "options": [ { "id": "b", "correct": true } ] }),
            "config",
            &mut issues,
        );
        assert!(issues[0].contains("not present"));
        merge_text(
            &mut config,
            &json!({ "options": [ { "id": "a", "correct": false } ] }),
            "config",
            &mut issues,
        );
        assert!(issues[1].contains("only change text"));
    }
}
