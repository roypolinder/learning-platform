use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

use lp_core::CompletionStrategy;
use serde_json::Value;

use crate::builtin::{self, CONDITIONAL, REUSABLE_SECTION, ReusableSectionConfig};
use crate::course::{Course, CourseStep, SectionOutline};
use crate::definition::{CompletionMode, CourseDefinition, StepDefinition};
use crate::error::{CourseIssue, CourseValidationError};
use crate::localization::{Locale, localize};
use crate::registry::{ExtensionRegistry, StepTypeRegistry};

const GENERIC_EXTENSION_TYPES: &[&str] = &["external_task", "extension_validation"];

struct PendingStep {
    path: String,
    section_id: String,
    definition: StepDefinition,
}

pub(crate) fn compile(
    definition: CourseDefinition,
    step_types: &StepTypeRegistry,
    extensions: &ExtensionRegistry,
) -> Result<Course, CourseValidationError> {
    let mut issues = Vec::new();
    validate_metadata(&definition, &mut issues);
    validate_glossary(&definition, &mut issues);
    validate_translations(&definition, &mut issues);

    let (pending, sections) = expand_sections(&definition, &mut issues);
    let mut steps = Vec::with_capacity(pending.len());
    let mut seen_ids = HashSet::new();
    let mut used_extensions = BTreeSet::new();
    let mut previous_id: Option<String> = None;

    for PendingStep {
        path,
        section_id,
        definition: step,
    } in pending
    {
        if !is_valid_identifier(&step.id, true) {
            issues.push(CourseIssue::new(
                format!("{path}.id"),
                "must be lowercase letters, digits, '-', '_' or '.'",
            ));
        }
        if !seen_ids.insert(step.id.clone()) {
            issues.push(CourseIssue::new(
                format!("{path}.id"),
                format!("duplicate step id `{}`", step.id),
            ));
        }

        let mut depends_on = step
            .depends_on
            .clone()
            .unwrap_or_else(|| previous_id.iter().cloned().collect());
        previous_id = Some(step.id.clone());

        let Some(entry) = step_types.get(&step.step_type) else {
            issues.push(CourseIssue::new(
                format!("{path}.type"),
                format!("unknown step type `{}`", step.step_type),
            ));
            continue;
        };

        let extension_id = resolve_extension(&path, &step, entry.provider, &mut issues);
        let mut completion = entry.spec.completion;
        if step.completion == Some(CompletionMode::Manual) {
            completion = CompletionStrategy::Acknowledge;
        }
        if matches!(
            completion,
            CompletionStrategy::Submission | CompletionStrategy::Event
        ) && extension_id.is_none()
        {
            issues.push(CourseIssue::new(
                path.clone(),
                format!(
                    "`{}` steps need an `extension` or `completion: manual`",
                    step.step_type
                ),
            ));
        }

        // Placeholders such as `{{theme.probes.catalog.count}}` only get their real value
        // (and type) when the course is rendered for a theme. Such steps are validated on
        // every rendered copy instead (see `catalog::check_renders`).
        let templated =
            has_theme_placeholders(&step.config) || has_theme_placeholders(&step.validation);
        if !templated {
            for message in builtin::validate(&step) {
                issues.push(CourseIssue::new(path.clone(), message));
            }
        }

        if let Some(extension_id) = &extension_id {
            match extensions.get(extension_id) {
                Some(extension) => {
                    used_extensions.insert(extension_id.clone());
                    if templated {
                        // Validated per rendered copy.
                    } else if let Err(messages) =
                        extension.validate_config(&step.step_type, &step.config, &step.validation)
                    {
                        issues.extend(
                            messages
                                .into_iter()
                                .map(|message| CourseIssue::new(path.clone(), message)),
                        );
                    }
                }
                None => issues.push(CourseIssue::new(
                    format!("{path}.extension"),
                    format!("extension `{extension_id}` is not registered"),
                )),
            }
        }

        if step.step_type == CONDITIONAL
            && let Some(condition) = builtin::condition_of(&step.config)
            && !depends_on.contains(&condition.step)
        {
            depends_on.push(condition.step);
        }

        steps.push(CourseStep {
            id: step.id,
            section_id,
            step_type: step.step_type,
            category: entry.spec.category,
            completion,
            extension: extension_id,
            title: step.title,
            description: step.description,
            body: step.body,
            config: normalize(step.config),
            validation: normalize(step.validation),
            depends_on,
            optional: step.optional,
            estimated_minutes: step.estimated_minutes,
            help: step.help,
        });
    }

    validate_dependencies(&steps, &mut issues);
    let extension_list = validate_declared_extensions(
        &definition.extensions,
        used_extensions,
        extensions,
        &mut issues,
    );
    let required_steps = resolve_required_steps(&definition, &steps, &mut issues);

    if issues.is_empty() {
        Ok(Course::new(
            definition,
            extension_list,
            sections,
            steps,
            required_steps,
        ))
    } else {
        Err(CourseValidationError {
            course_id: definition.id,
            issues,
        })
    }
}

fn validate_metadata(definition: &CourseDefinition, issues: &mut Vec<CourseIssue>) {
    if !is_valid_identifier(&definition.id, false) {
        issues.push(CourseIssue::new(
            "id",
            "must be lowercase letters, digits and '-'",
        ));
    }
    if !is_semver(&definition.version) {
        issues.push(CourseIssue::new(
            "version",
            "must be MAJOR.MINOR.PATCH, e.g. 1.0.0",
        ));
    }
    for (field, value) in [
        ("title", &definition.title),
        ("summary", &definition.summary),
        ("description", &definition.description),
        ("category", &definition.category),
    ] {
        if value.trim().is_empty() {
            issues.push(CourseIssue::new(field, "must not be empty"));
        }
    }
    if definition.estimated_minutes == 0 {
        issues.push(CourseIssue::new("estimated_minutes", "must be positive"));
    }
    if definition.sections.is_empty() {
        issues.push(CourseIssue::new(
            "sections",
            "at least one section is required",
        ));
    }
}

fn validate_glossary(definition: &CourseDefinition, issues: &mut Vec<CourseIssue>) {
    let mut terms = HashSet::new();
    for (index, entry) in definition.glossary.iter().enumerate() {
        if entry.term.trim().is_empty() || entry.definition.trim().is_empty() {
            issues.push(CourseIssue::new(
                format!("glossary[{index}]"),
                "term and definition must not be empty",
            ));
        }
        if !terms.insert(entry.term.to_lowercase()) {
            issues.push(CourseIssue::new(
                format!("glossary[{index}].term"),
                format!("duplicate term `{}`", entry.term),
            ));
        }
    }
}

/// Translations are checked structurally here; rendering checks the localized steps.
fn validate_translations(definition: &CourseDefinition, issues: &mut Vec<CourseIssue>) {
    for locale in definition.translations.keys() {
        if *locale == Locale::SOURCE {
            issues.push(CourseIssue::new(
                format!("translations.{locale}"),
                "courses are written in this language; no translation needed",
            ));
        } else if let Err(problems) = localize(definition, *locale) {
            issues.extend(problems);
        }
    }
}

/// Flattens sections and inlines `reusable_section` steps. Inlined step ids are
/// prefixed with the referencing step id so a shared section can be used twice.
fn expand_sections(
    definition: &CourseDefinition,
    issues: &mut Vec<CourseIssue>,
) -> (Vec<PendingStep>, Vec<SectionOutline>) {
    let mut pending = Vec::new();
    let mut sections = Vec::with_capacity(definition.sections.len());
    let mut section_ids = HashSet::new();

    for (section_index, section) in definition.sections.iter().enumerate() {
        let section_path = format!("sections[{section_index}]");
        if !section_ids.insert(section.id.as_str()) {
            issues.push(CourseIssue::new(
                format!("{section_path}.id"),
                format!("duplicate section id `{}`", section.id),
            ));
        }
        if section.steps.is_empty() {
            issues.push(CourseIssue::new(
                format!("{section_path}.steps"),
                "a section needs at least one step",
            ));
        }
        let first_pending = pending.len();

        for (step_index, step) in section.steps.iter().enumerate() {
            let path = format!("{section_path}.steps[{step_index}]");
            if step.step_type != REUSABLE_SECTION {
                pending.push(PendingStep {
                    path,
                    section_id: section.id.clone(),
                    definition: step.clone(),
                });
                continue;
            }

            let reference =
                match lp_core::parse_config::<ReusableSectionConfig>("config", &step.config) {
                    Ok(config) => config.section,
                    Err(messages) => {
                        issues.extend(
                            messages
                                .into_iter()
                                .map(|m| CourseIssue::new(path.clone(), m)),
                        );
                        continue;
                    }
                };
            let Some(shared) = definition.shared_sections.get(&reference) else {
                issues.push(CourseIssue::new(
                    format!("{path}.config.section"),
                    format!("shared section `{reference}` does not exist"),
                ));
                continue;
            };

            for (inner_index, inner) in shared.steps.iter().enumerate() {
                let inner_path =
                    format!("{path} -> shared_sections.{reference}.steps[{inner_index}]");
                if inner.step_type == REUSABLE_SECTION {
                    issues.push(CourseIssue::new(
                        inner_path,
                        "shared sections cannot nest reusable sections",
                    ));
                    continue;
                }
                let mut expanded = inner.clone();
                expanded.id = format!("{}.{}", step.id, inner.id);
                expanded.depends_on = match &inner.depends_on {
                    Some(dependencies) => Some(
                        dependencies
                            .iter()
                            .map(|dependency| format!("{}.{dependency}", step.id))
                            .collect(),
                    ),
                    None if inner_index == 0 => step.depends_on.clone(),
                    None => None,
                };
                pending.push(PendingStep {
                    path: inner_path,
                    section_id: section.id.clone(),
                    definition: expanded,
                });
            }
        }

        sections.push(SectionOutline {
            id: section.id.clone(),
            title: section.title.clone(),
            description: section.description.clone(),
            step_ids: pending[first_pending..]
                .iter()
                .map(|step| step.definition.id.clone())
                .collect(),
        });
    }
    (pending, sections)
}

fn resolve_extension(
    path: &str,
    step: &StepDefinition,
    provider: Option<&'static str>,
    issues: &mut Vec<CourseIssue>,
) -> Option<String> {
    match (provider, &step.extension) {
        (Some(provider), None) => Some(provider.to_owned()),
        (Some(provider), Some(explicit)) if explicit == provider => Some(provider.to_owned()),
        (Some(provider), Some(explicit)) => {
            issues.push(CourseIssue::new(
                format!("{path}.extension"),
                format!(
                    "`{}` steps are handled by `{provider}`, not `{explicit}`",
                    step.step_type
                ),
            ));
            None
        }
        (None, Some(explicit)) if GENERIC_EXTENSION_TYPES.contains(&step.step_type.as_str()) => {
            Some(explicit.clone())
        }
        (None, Some(_)) => {
            issues.push(CourseIssue::new(
                format!("{path}.extension"),
                format!("`{}` steps do not use an extension", step.step_type),
            ));
            None
        }
        (None, None) => None,
    }
}

fn validate_dependencies(steps: &[CourseStep], issues: &mut Vec<CourseIssue>) {
    let positions: HashMap<&str, usize> = steps
        .iter()
        .enumerate()
        .map(|(position, step)| (step.id.as_str(), position))
        .collect();
    let mut indegree = vec![0usize; steps.len()];
    let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); steps.len()];

    for (position, step) in steps.iter().enumerate() {
        for dependency in &step.depends_on {
            match positions.get(dependency.as_str()) {
                Some(&dependency_position) if dependency_position == position => {
                    issues.push(CourseIssue::new(
                        format!("steps.{}.depends_on", step.id),
                        "a step cannot depend on itself",
                    ));
                }
                Some(&dependency_position) => {
                    indegree[position] += 1;
                    dependents[dependency_position].push(position);
                }
                None => issues.push(CourseIssue::new(
                    format!("steps.{}.depends_on", step.id),
                    format!("unknown step `{dependency}`"),
                )),
            }
        }
    }

    let mut queue: VecDeque<usize> = (0..steps.len()).filter(|i| indegree[*i] == 0).collect();
    let mut visited = 0;
    while let Some(position) = queue.pop_front() {
        visited += 1;
        for &dependent in &dependents[position] {
            indegree[dependent] -= 1;
            if indegree[dependent] == 0 {
                queue.push_back(dependent);
            }
        }
    }
    if visited < steps.len() {
        let cyclic: Vec<&str> = steps
            .iter()
            .enumerate()
            .filter(|(position, _)| indegree[*position] > 0)
            .map(|(_, step)| step.id.as_str())
            .collect();
        issues.push(CourseIssue::new(
            "steps",
            format!("dependency cycle involving: {}", cyclic.join(", ")),
        ));
    }
}

fn validate_declared_extensions(
    declared: &[String],
    used: BTreeSet<String>,
    registry: &ExtensionRegistry,
    issues: &mut Vec<CourseIssue>,
) -> Vec<String> {
    for (index, id) in declared.iter().enumerate() {
        if registry.get(id).is_none() {
            issues.push(CourseIssue::new(
                format!("extensions[{index}]"),
                format!("extension `{id}` is not registered"),
            ));
        }
    }
    if !declared.is_empty() {
        for id in used.iter().filter(|id| !declared.contains(id)) {
            issues.push(CourseIssue::new(
                "extensions",
                format!("steps use `{id}` but it is not declared"),
            ));
        }
    }
    let mut all: BTreeSet<String> = declared.iter().cloned().collect();
    all.extend(used);
    all.into_iter().collect()
}

fn resolve_required_steps(
    definition: &CourseDefinition,
    steps: &[CourseStep],
    issues: &mut Vec<CourseIssue>,
) -> Vec<String> {
    match &definition.completion.required_steps {
        Some(required) => {
            for id in required {
                if !steps.iter().any(|step| &step.id == id) {
                    issues.push(CourseIssue::new(
                        "completion.required_steps",
                        format!("unknown step `{id}`"),
                    ));
                }
            }
            required.clone()
        }
        None => steps
            .iter()
            .filter(|step| !step.optional)
            .map(|step| step.id.clone())
            .collect(),
    }
}

fn has_theme_placeholders(value: &Value) -> bool {
    match value {
        Value::String(text) => text.contains("{{theme.") || text.contains("{{ theme."),
        Value::Array(items) => items.iter().any(has_theme_placeholders),
        Value::Object(fields) => fields.values().any(has_theme_placeholders),
        _ => false,
    }
}

fn normalize(value: Value) -> Value {
    if value.is_null() {
        Value::Object(Default::default())
    } else {
        value
    }
}

fn is_valid_identifier(value: &str, allow_step_punctuation: bool) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    value.len() <= 120
        && (first.is_ascii_lowercase() || first.is_ascii_digit())
        && chars.all(|c| {
            c.is_ascii_lowercase()
                || c.is_ascii_digit()
                || c == '-'
                || (allow_step_punctuation && (c == '_' || c == '.'))
        })
}

fn is_semver(value: &str) -> bool {
    let parts: Vec<&str> = value.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
}
