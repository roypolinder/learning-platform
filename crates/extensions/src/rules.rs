//! Assertions on JSON responses from learner services. Hidden test cases use them to
//! check behaviour (rankings, extracted values, traces) rather than exact output, so
//! any correct implementation passes.
//!
//! String comparisons are trimmed and case-insensitive; numbers compare by value.

use serde::Deserialize;
use serde_json::Value;

/// Placeholder replaced with a value that is fresh for every check run, so learners
/// cannot hardcode answers and repeated runs don't collide.
pub const NONCE_TOKEN: &str = "{{nonce}}";

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JsonRule {
    /// JSON pointer; a `*` segment matches every key or index.
    pub pointer: String,
    /// Pointer applied to each list item before list or value assertions.
    #[serde(default)]
    pub field: Option<String>,
    /// Only the first `k` list items are considered.
    #[serde(default)]
    pub top_k: Option<usize>,

    #[serde(default)]
    pub equals: Option<Value>,
    #[serde(default)]
    pub one_of: Vec<Value>,
    #[serde(default)]
    pub contains: Option<String>,
    #[serde(default)]
    pub contains_any: Vec<String>,
    #[serde(default)]
    pub contains_all: Vec<String>,
    #[serde(default)]
    pub not_contains_any: Vec<String>,
    #[serde(default)]
    pub approx: Option<f64>,
    #[serde(default)]
    pub tolerance: Option<f64>,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default)]
    pub min_length: Option<usize>,
    #[serde(default, rename = "type")]
    pub json_type: Option<String>,

    #[serde(default)]
    pub min_items: Option<usize>,
    #[serde(default)]
    pub max_items: Option<usize>,
    #[serde(default)]
    pub includes: Option<Value>,
    #[serde(default)]
    pub includes_all: Vec<Value>,
    #[serde(default)]
    pub excludes: Option<Value>,
    #[serde(default)]
    pub all_equal: Option<Value>,
    /// Values that must appear in this order (other items may sit in between).
    #[serde(default)]
    pub in_order: Vec<Value>,
    /// `asc` or `desc`; items must be numbers.
    #[serde(default)]
    pub sorted: Option<String>,

    /// Passes only when nothing (or only `null`) is found.
    #[serde(default)]
    pub absent: bool,
}

const TYPES: &[&str] = &[
    "array", "object", "string", "number", "integer", "boolean", "null",
];

impl JsonRule {
    pub fn issues(&self, path: &str) -> Vec<String> {
        let mut issues = Vec::new();
        if !self.pointer.is_empty() && !self.pointer.starts_with('/') {
            issues.push(format!("{path}.pointer must be a JSON pointer"));
        }
        if self.field.as_deref().is_some_and(|f| !f.starts_with('/')) {
            issues.push(format!("{path}.field must be a JSON pointer"));
        }
        if self
            .json_type
            .as_deref()
            .is_some_and(|t| !TYPES.contains(&t))
        {
            issues.push(format!("{path}.type must be one of {}", TYPES.join(", ")));
        }
        if self
            .sorted
            .as_deref()
            .is_some_and(|s| s != "asc" && s != "desc")
        {
            issues.push(format!("{path}.sorted must be `asc` or `desc`"));
        }
        if self.tolerance.is_some() && self.approx.is_none() {
            issues.push(format!("{path}.tolerance needs approx"));
        }
        issues
    }

    fn uses_list_rules(&self) -> bool {
        self.top_k.is_some()
            || self.min_items.is_some()
            || self.max_items.is_some()
            || self.includes.is_some()
            || !self.includes_all.is_empty()
            || self.excludes.is_some()
            || self.all_equal.is_some()
            || !self.in_order.is_empty()
            || self.sorted.is_some()
    }

    fn uses_value_rules(&self) -> bool {
        self.equals.is_some()
            || !self.one_of.is_empty()
            || self.contains.is_some()
            || !self.contains_any.is_empty()
            || !self.contains_all.is_empty()
            || !self.not_contains_any.is_empty()
            || self.approx.is_some()
            || self.min.is_some()
            || self.max.is_some()
            || self.min_length.is_some()
            || self.json_type.is_some()
    }

    /// `Err` carries a short, learner-facing reason.
    pub fn check(&self, document: &Value, nonce: &str) -> Result<(), String> {
        let present: Vec<&Value> = select(document, &self.pointer)
            .into_iter()
            .filter(|value| !value.is_null())
            .collect();
        let name = if self.pointer.is_empty() {
            "response".to_owned()
        } else {
            format!("`{}`", self.pointer)
        };
        if self.absent {
            return if present.is_empty() {
                Ok(())
            } else {
                Err(format!("{name} should not be set"))
            };
        }
        let Some(first) = present.first() else {
            return Err(format!("{name} is missing"));
        };
        let wildcard = self.pointer.split('/').any(|segment| segment == "*");
        let mut items: Option<Vec<&Value>> = if wildcard {
            Some(present.clone())
        } else {
            first.as_array().map(|list| list.iter().collect())
        };
        if let (Some(k), Some(list)) = (self.top_k, items.as_mut()) {
            list.truncate(k);
        }
        if let Some(field) = &self.field {
            items = items.map(|list| list.into_iter().filter_map(|i| i.pointer(field)).collect());
        }

        let subst = |value: &Value| substitute(value, nonce);
        if self.uses_list_rules() {
            let list = items
                .as_ref()
                .ok_or_else(|| format!("{name} is not a list"))?;
            let scope = match self.top_k {
                Some(k) => format!("the top {k} of {name}"),
                None => name.clone(),
            };
            if let Some(min) = self.min_items
                && list.len() < min
            {
                return Err(format!(
                    "{name} has {} items, expected at least {min}",
                    list.len()
                ));
            }
            if let Some(max) = self.max_items
                && list.len() > max
            {
                return Err(format!(
                    "{name} has {} items, expected at most {max}",
                    list.len()
                ));
            }
            for expected in self.includes.iter().chain(&self.includes_all) {
                let expected = subst(expected);
                if !list.iter().any(|item| loose_eq(item, &expected)) {
                    return Err(format!("{} not found in {scope}", short(&expected)));
                }
            }
            if let Some(unwanted) = &self.excludes {
                let unwanted = subst(unwanted);
                if list.iter().any(|item| loose_eq(item, &unwanted)) {
                    return Err(format!("{} should not be in {scope}", short(&unwanted)));
                }
            }
            if let Some(expected) = &self.all_equal {
                let expected = subst(expected);
                if list.is_empty() {
                    return Err(format!("{scope} is empty"));
                }
                if let Some(other) = list.iter().find(|item| !loose_eq(item, &expected)) {
                    return Err(format!(
                        "expected only {} in {scope}, found {}",
                        short(&expected),
                        short(other)
                    ));
                }
            }
            if !self.in_order.is_empty() {
                let mut remaining = list.iter();
                for expected in &self.in_order {
                    let expected = subst(expected);
                    if !remaining.any(|item| loose_eq(item, &expected)) {
                        return Err(format!(
                            "{scope} is missing {} in the expected order",
                            short(&expected)
                        ));
                    }
                }
            }
            if let Some(order) = self.sorted.as_deref() {
                let numbers: Option<Vec<f64>> = list.iter().map(|item| item.as_f64()).collect();
                let numbers = numbers.ok_or_else(|| format!("{scope} must contain numbers"))?;
                let ok = numbers.windows(2).all(|pair| match order {
                    "asc" => pair[0] <= pair[1],
                    _ => pair[0] >= pair[1],
                });
                if !ok {
                    return Err(format!("{scope} is not sorted {order}ending"));
                }
            }
        }

        if self.uses_value_rules() {
            let targets: Vec<&Value> = if wildcard || self.field.is_some() {
                items.unwrap_or_default()
            } else {
                vec![*first]
            };
            if targets.is_empty() {
                return Err(format!("{name} has nothing to check"));
            }
            for target in targets {
                self.check_value(target, &name, nonce)?;
            }
        }
        Ok(())
    }

    fn check_value(&self, value: &Value, name: &str, nonce: &str) -> Result<(), String> {
        let got = || short(value);
        if let Some(expected) = &self.equals {
            let expected = substitute(expected, nonce);
            if !loose_eq(value, &expected) {
                return Err(format!(
                    "{name} is {}, expected {}",
                    got(),
                    short(&expected)
                ));
            }
        }
        if !self.one_of.is_empty()
            && !self
                .one_of
                .iter()
                .any(|option| loose_eq(value, &substitute(option, nonce)))
        {
            return Err(format!(
                "{name} is {}, not one of the expected values",
                got()
            ));
        }
        if let Some(kind) = self.json_type.as_deref() {
            let ok = match kind {
                "array" => value.is_array(),
                "object" => value.is_object(),
                "string" => value.is_string(),
                "number" => value.is_number(),
                "integer" => value.is_i64() || value.is_u64(),
                "boolean" => value.is_boolean(),
                _ => value.is_null(),
            };
            if !ok {
                return Err(format!("{name} should be a {kind}"));
            }
        }
        let text = text_of(value).to_lowercase();
        if let Some(needle) = &self.contains
            && !text.contains(&needle.replace(NONCE_TOKEN, nonce).to_lowercase())
        {
            return Err(format!("{name} does not mention “{needle}”"));
        }
        if !self.contains_any.is_empty()
            && !self
                .contains_any
                .iter()
                .any(|needle| text.contains(&needle.to_lowercase()))
        {
            return Err(format!(
                "{name} does not mention “{}”",
                self.contains_any[0]
            ));
        }
        if let Some(missing) = self
            .contains_all
            .iter()
            .find(|needle| !text.contains(&needle.to_lowercase()))
        {
            return Err(format!("{name} does not mention “{missing}”"));
        }
        if let Some(found) = self
            .not_contains_any
            .iter()
            .find(|needle| text.contains(&needle.to_lowercase()))
        {
            return Err(format!("{name} should not mention “{found}”"));
        }
        if let Some(min_length) = self.min_length
            && text.chars().count() < min_length
        {
            return Err(format!("{name} should be at least {min_length} characters"));
        }
        if self.approx.is_some() || self.min.is_some() || self.max.is_some() {
            let number = number_of(value).ok_or_else(|| format!("{name} should be a number"))?;
            if let Some(expected) = self.approx {
                let tolerance = self.tolerance.unwrap_or(0.01);
                if (number - expected).abs() > tolerance {
                    return Err(format!("{name} is {number}, expected about {expected}"));
                }
            }
            if self.min.is_some_and(|min| number < min) || self.max.is_some_and(|max| number > max)
            {
                return Err(format!("{name} is {number}, outside the expected range"));
            }
        }
        Ok(())
    }
}

pub fn select<'a>(document: &'a Value, pointer: &str) -> Vec<&'a Value> {
    let mut current = vec![document];
    for raw in pointer.split('/').skip(1) {
        let segment = raw.replace("~1", "/").replace("~0", "~");
        current = current
            .into_iter()
            .flat_map(|value| -> Vec<&Value> {
                match (value, segment.as_str()) {
                    (Value::Object(map), "*") => map.values().collect(),
                    (Value::Array(items), "*") => items.iter().collect(),
                    (Value::Object(map), key) => map.get(key).into_iter().collect(),
                    (Value::Array(items), index) => index
                        .parse::<usize>()
                        .ok()
                        .and_then(|index| items.get(index))
                        .into_iter()
                        .collect(),
                    _ => Vec::new(),
                }
            })
            .collect();
    }
    current
}

pub fn substitute(value: &Value, nonce: &str) -> Value {
    match value {
        Value::String(text) if text.contains(NONCE_TOKEN) => {
            Value::String(text.replace(NONCE_TOKEN, nonce))
        }
        Value::Array(items) => items.iter().map(|item| substitute(item, nonce)).collect(),
        Value::Object(map) => map
            .iter()
            .map(|(key, item)| (key.clone(), substitute(item, nonce)))
            .collect(),
        other => other.clone(),
    }
}

fn loose_eq(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::String(a), Value::String(b)) => a.trim().eq_ignore_ascii_case(b.trim()),
        (Value::Number(_), Value::Number(_)) => actual
            .as_f64()
            .zip(expected.as_f64())
            .is_some_and(|(a, b)| (a - b).abs() < 1e-9),
        _ => actual == expected,
    }
}

fn text_of(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// Accepts numbers and numeric strings such as "€1,250.50", which models often return.
fn number_of(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => {
            let cleaned: String = text
                .chars()
                .filter(|c| c.is_ascii_digit() || matches!(c, '.' | '-'))
                .collect();
            cleaned.parse().ok()
        }
        _ => None,
    }
}

fn short(value: &Value) -> String {
    let text = match value {
        Value::String(text) => format!("“{text}”"),
        other => other.to_string(),
    };
    if text.chars().count() > 80 {
        format!("{}…", text.chars().take(80).collect::<String>())
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn rule(value: Value) -> JsonRule {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn ranks_and_filters_lists() {
        let doc = json!({ "results": [
            { "id": "d1", "category": "Finance", "score": 0.9 },
            { "id": "d7", "category": "hr", "score": 0.5 },
            { "id": "probe-ab12", "category": "facilities", "score": 0.4 }
        ]});
        let top = rule(
            json!({ "pointer": "/results", "top_k": 1, "field": "/category", "includes": "finance" }),
        );
        assert!(top.check(&doc, "x").is_ok());
        let wrong = rule(
            json!({ "pointer": "/results", "top_k": 1, "field": "/category", "includes": "hr" }),
        );
        assert!(wrong.check(&doc, "x").is_err());
        let nonce =
            rule(json!({ "pointer": "/results", "field": "/id", "includes": "probe-{{nonce}}" }));
        assert!(nonce.check(&doc, "ab12").is_ok());
        assert!(nonce.check(&doc, "zz99").is_err());
        let sorted = rule(json!({ "pointer": "/results", "field": "/score", "sorted": "desc" }));
        assert!(sorted.check(&doc, "x").is_ok());
        let only =
            rule(json!({ "pointer": "/results", "field": "/category", "all_equal": "finance" }));
        assert!(only.check(&doc, "x").is_err());
    }

    #[test]
    fn checks_values_wildcards_and_order() {
        let doc = json!({
            "answer": "You get 26 days of leave.",
            "total": "€1,250.50",
            "trace": [{ "stage": "classify" }, { "stage": "extract" }, { "stage": "route" }],
            "tools": [{ "name": "a", "input_schema": { "type": "object" } }]
        });
        assert!(
            rule(json!({ "pointer": "/answer", "contains_any": ["twenty-six", "26"] }))
                .check(&doc, "")
                .is_ok()
        );
        assert!(
            rule(json!({ "pointer": "/answer", "not_contains_any": ["banana"] }))
                .check(&doc, "")
                .is_ok()
        );
        assert!(
            rule(json!({ "pointer": "/total", "approx": 1250.5 }))
                .check(&doc, "")
                .is_ok()
        );
        assert!(
            rule(
                json!({ "pointer": "/trace", "field": "/stage", "in_order": ["classify", "route"] })
            )
            .check(&doc, "")
            .is_ok()
        );
        assert!(
            rule(
                json!({ "pointer": "/trace", "field": "/stage", "in_order": ["route", "classify"] })
            )
            .check(&doc, "")
            .is_err()
        );
        assert!(
            rule(json!({ "pointer": "/tools/*/input_schema/type", "equals": "object" }))
                .check(&doc, "")
                .is_ok()
        );
        assert!(
            rule(json!({ "pointer": "/missing", "absent": true }))
                .check(&doc, "")
                .is_ok()
        );
        assert!(
            rule(json!({ "pointer": "/answer", "absent": true }))
                .check(&doc, "")
                .is_err()
        );
    }
}
