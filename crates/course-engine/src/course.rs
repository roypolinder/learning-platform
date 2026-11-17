use std::collections::HashMap;

use lp_core::{CompletionStrategy, StepCategory};
use serde_json::Value;

use crate::definition::{CourseDefinition, Difficulty, GlossaryEntry, PrerequisiteDefinition};

/// A validated, flattened course ready for execution.
#[derive(Debug, Clone)]
pub struct Course {
    pub id: String,
    pub version: String,
    pub title: String,
    pub summary: String,
    pub description: String,
    pub category: String,
    pub difficulty: Difficulty,
    pub estimated_minutes: u32,
    pub order: Option<u32>,
    pub help: Option<String>,
    pub tags: Vec<String>,
    pub objectives: Vec<String>,
    pub prerequisites: Vec<PrerequisiteDefinition>,
    pub glossary: Vec<GlossaryEntry>,
    pub extensions: Vec<String>,
    pub sections: Vec<SectionOutline>,
    pub steps: Vec<CourseStep>,
    pub required_steps: Vec<String>,
    pub definition: CourseDefinition,
    index: HashMap<String, usize>,
}

#[derive(Debug, Clone)]
pub struct SectionOutline {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub step_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CourseStep {
    pub id: String,
    pub section_id: String,
    pub step_type: String,
    pub category: StepCategory,
    pub completion: CompletionStrategy,
    pub extension: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub body: Option<String>,
    pub config: Value,
    pub validation: Value,
    pub depends_on: Vec<String>,
    pub optional: bool,
    pub estimated_minutes: Option<u32>,
    pub help: Option<String>,
}

impl Course {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        definition: CourseDefinition,
        extensions: Vec<String>,
        sections: Vec<SectionOutline>,
        steps: Vec<CourseStep>,
        required_steps: Vec<String>,
    ) -> Self {
        let index = steps
            .iter()
            .enumerate()
            .map(|(position, step)| (step.id.clone(), position))
            .collect();
        Self {
            id: definition.id.clone(),
            version: definition.version.clone(),
            title: definition.title.clone(),
            summary: definition.summary.clone(),
            description: definition.description.clone(),
            category: definition.category.clone(),
            difficulty: definition.difficulty,
            estimated_minutes: definition.estimated_minutes,
            order: definition.order,
            help: definition.help.clone(),
            tags: definition.tags.clone(),
            objectives: definition.objectives.clone(),
            prerequisites: definition.prerequisites.clone(),
            glossary: definition.glossary.clone(),
            extensions,
            sections,
            steps,
            required_steps,
            definition,
            index,
        }
    }

    pub fn step(&self, step_id: &str) -> Option<&CourseStep> {
        self.index
            .get(step_id)
            .map(|position| &self.steps[*position])
    }

    pub fn position(&self, step_id: &str) -> Option<usize> {
        self.index.get(step_id).copied()
    }

    pub fn first_step(&self) -> Option<&CourseStep> {
        self.steps.first()
    }
}
