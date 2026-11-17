//! Renders a course for one learner: their language and their project theme.
//!
//! `{{theme.path.to.value}}` placeholders may appear in any text, config or validation
//! value. Inside longer text they are replaced by the value's text; a string that is
//! *only* a placeholder takes the value as-is, so lists and numbers keep their type:
//!
//! ```yaml
//! request: { path: /api/search, query: { q: "{{theme.probes.search.paraphrase.query}}" } }
//! expect: { json: [ { pointer: /results, top_k: 1, field: /id,
//!                     includes: "{{theme.probes.search.paraphrase.expected}}" } ] }
//! ```
//!
//! Other `{{…}}` placeholders (such as `{{nonce}}`) are left for the extensions.

use std::collections::BTreeSet;

use serde_json::Value;

use crate::definition::CourseDefinition;
use crate::error::CourseIssue;
use crate::localization::{Locale, localize};
use crate::theme::Theme;

const PREFIX: &str = "theme.";
/// Shown for values the neutral theme does not define, e.g. hidden test data.
const UNRESOLVED: &str = "…";

#[derive(Debug, Clone, Copy)]
pub struct Audience<'a> {
    pub locale: Locale,
    pub theme: &'a Theme,
}

/// Localizes `definition` and fills in theme placeholders. With a selectable theme every
/// placeholder must resolve; with the neutral theme unknown ones become "…".
pub fn personalize(
    definition: &CourseDefinition,
    audience: Audience<'_>,
) -> Result<CourseDefinition, Vec<CourseIssue>> {
    let localized = localize(definition, audience.locale)?;
    let mut value = serde_json::to_value(&localized)
        .map_err(|error| vec![CourseIssue::new("", error.to_string())])?;
    let mut missing = BTreeSet::new();
    substitute(
        &mut value,
        audience.theme.scope(audience.locale),
        &mut missing,
    );
    if audience.theme.is_selectable() && !missing.is_empty() {
        return Err(missing
            .into_iter()
            .map(|name| {
                CourseIssue::new(
                    format!("theme `{}` ({})", audience.theme.id(), audience.locale),
                    format!("`{{{{{name}}}}}` is not defined"),
                )
            })
            .collect());
    }
    serde_json::from_value(value).map_err(|error| {
        vec![CourseIssue::new(
            format!("theme `{}`", audience.theme.id()),
            format!("a placeholder produced an invalid value: {error}"),
        )]
    })
}

/// Replaces placeholders throughout `value`, recording names that did not resolve.
pub fn substitute(value: &mut Value, scope: &Value, missing: &mut BTreeSet<String>) {
    match value {
        Value::String(text) => {
            if let Some(replacement) = render_text(text, scope, missing) {
                *value = replacement;
            }
        }
        Value::Array(items) => items
            .iter_mut()
            .for_each(|item| substitute(item, scope, missing)),
        Value::Object(fields) => fields
            .values_mut()
            .for_each(|field| substitute(field, scope, missing)),
        _ => {}
    }
}

/// `None` when `text` contains no theme placeholder.
fn render_text(text: &str, scope: &Value, missing: &mut BTreeSet<String>) -> Option<Value> {
    if !text.contains("{{") {
        return None;
    }
    let placeholders = find_placeholders(text);
    if placeholders.is_empty() {
        return None;
    }
    if let [(start, end, name)] = placeholders.as_slice()
        && *start == 0
        && *end == text.len()
    {
        return Some(match lookup(scope, name) {
            Some(found) => found.clone(),
            None => {
                missing.insert(format!("{PREFIX}{name}"));
                Value::String(UNRESOLVED.into())
            }
        });
    }
    let mut rendered = String::with_capacity(text.len());
    let mut cursor = 0;
    for (start, end, name) in placeholders {
        rendered.push_str(&text[cursor..start]);
        match lookup(scope, &name) {
            Some(Value::String(found)) => rendered.push_str(found),
            Some(Value::Null) | None => {
                missing.insert(format!("{PREFIX}{name}"));
                rendered.push_str(UNRESOLVED);
            }
            Some(other) => rendered.push_str(&other.to_string()),
        }
        cursor = end;
    }
    rendered.push_str(&text[cursor..]);
    Some(Value::String(rendered))
}

/// `(start, end, path after "theme.")` for every `{{theme.…}}` in `text`.
fn find_placeholders(text: &str) -> Vec<(usize, usize, String)> {
    let mut found = Vec::new();
    let mut offset = 0;
    while let Some(open) = text[offset..].find("{{") {
        let start = offset + open;
        let Some(close) = text[start + 2..].find("}}") else {
            break;
        };
        let end = start + 2 + close + 2;
        let inner = text[start + 2..end - 2].trim();
        if let Some(path) = inner.strip_prefix(PREFIX)
            && !path.is_empty()
            && path
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        {
            found.push((start, end, path.to_owned()));
        }
        offset = end;
    }
    found
}

fn lookup<'a>(scope: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.').try_fold(scope, |value, key| match value {
        Value::Object(fields) => fields.get(key),
        Value::Array(items) => key.parse::<usize>().ok().and_then(|i| items.get(i)),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn scope() -> Value {
        json!({
            "item": "bike",
            "probes": { "search": { "query": "fiets voor moeder", "expected": ["bike-child-seat"], "count": 3 } }
        })
    }

    #[test]
    fn substitutes_text_and_keeps_types_for_whole_values() {
        let mut value = json!({
            "body": "Find a {{theme.item}} with {{ theme.probes.search.count }} results",
            "q": "{{theme.probes.search.query}}",
            "includes": "{{theme.probes.search.expected}}",
            "nonce": "probe-{{nonce}}"
        });
        let mut missing = BTreeSet::new();
        substitute(&mut value, &scope(), &mut missing);
        assert!(missing.is_empty());
        assert_eq!(value["body"], "Find a bike with 3 results");
        assert_eq!(value["q"], "fiets voor moeder");
        assert_eq!(value["includes"], json!(["bike-child-seat"]));
        assert_eq!(value["nonce"], "probe-{{nonce}}");
    }

    #[test]
    fn reports_unknown_placeholders() {
        let mut value = json!("A {{theme.missing}} and {{theme.probes.nope}}");
        let mut missing = BTreeSet::new();
        substitute(&mut value, &scope(), &mut missing);
        assert_eq!(value, json!("A … and …"));
        assert_eq!(missing.len(), 2);
    }
}
