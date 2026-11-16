//! Learner-facing projections of compiled courses. Private validation rules and
//! anything an extension redacts never leave the engine through these types.

use lp_core::{CompletionStrategy, StepCategory};
use serde::Serialize;
use serde_json::Value;

use crate::course::{Course, CourseStep};
use crate::definition::{Difficulty, PrerequisiteDefinition};
use crate::runtime::CourseEngine;

#[derive(Debug, Clone, Serialize)]
pub struct CourseSummaryView {
    pub id: String,
    pub version: String,
    pub title: String,
    pub summary: String,
    pub category: String,
    pub difficulty: Difficulty,
    pub estimated_minutes: u32,
    pub order: Option<u32>,
    pub tags: Vec<String>,
    pub section_count: usize,
    pub step_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct CourseDetailView {
    #[serde(flatten)]
    pub summary: CourseSummaryView,
    pub description: String,
    pub help: Option<String>,
    pub objectives: Vec<String>,
    pub prerequisites: Vec<PrerequisiteDefinition>,
    pub extensions: Vec<String>,
    pub sections: Vec<SectionView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SectionView {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub steps: Vec<StepView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StepView {
    pub id: String,
    pub section_id: String,
    #[serde(rename = "type")]
    pub step_type: String,
    pub category: StepCategory,
    pub completion: CompletionStrategy,
    pub title: String,
    pub description: Option<String>,
    pub body: Option<String>,
    pub extension: Option<String>,
    pub config: Value,
    pub submission_schema: Option<Value>,
    /// Requests the learner's browser runs against their own service before submitting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_task: Option<Value>,
    pub depends_on: Vec<String>,
    pub optional: bool,
    pub estimated_minutes: Option<u32>,
    pub help: Option<String>,
}

pub fn course_summary(course: &Course) -> CourseSummaryView {
    CourseSummaryView {
        id: course.id.clone(),
        version: course.version.clone(),
        title: course.title.clone(),
        summary: course.summary.clone(),
        category: course.category.clone(),
        difficulty: course.difficulty,
        estimated_minutes: course.estimated_minutes,
        order: course.order,
        tags: course.tags.clone(),
        section_count: course.sections.len(),
        step_count: course.steps.len(),
    }
}

pub fn course_detail(engine: &CourseEngine, course: &Course) -> CourseDetailView {
    let sections = course
        .sections
        .iter()
        .map(|section| SectionView {
            id: section.id.clone(),
            title: section.title.clone(),
            description: section.description.clone(),
            steps: section
                .step_ids
                .iter()
                .filter_map(|id| course.step(id))
                .map(|step| step_view(engine, step))
                .collect(),
        })
        .collect();
    CourseDetailView {
        summary: course_summary(course),
        description: course.description.clone(),
        help: course.help.clone(),
        objectives: course.objectives.clone(),
        prerequisites: course.prerequisites.clone(),
        extensions: course.extensions.clone(),
        sections,
    }
}

pub fn step_view(engine: &CourseEngine, step: &CourseStep) -> StepView {
    let extension = step
        .extension
        .as_deref()
        .and_then(|id| engine.extensions().get(id));
    let config = match extension {
        Some(extension) => extension.public_config(&step.step_type, &step.config),
        None => step.config.clone(),
    };
    let submission_schema =
        extension.and_then(|extension| extension.submission_schema(&step.step_type, &step.config));
    let client_task = extension.and_then(|extension| {
        extension.client_task(&step.step_type, &step.config, &step.validation)
    });
    StepView {
        id: step.id.clone(),
        section_id: step.section_id.clone(),
        step_type: step.step_type.clone(),
        category: step.category,
        completion: step.completion,
        title: step.title.clone(),
        description: step.description.clone(),
        body: step.body.clone(),
        extension: step.extension.clone(),
        config,
        submission_schema,
        client_task,
        depends_on: step.depends_on.clone(),
        optional: step.optional,
        estimated_minutes: step.estimated_minutes,
        help: step.help.clone(),
    }
}
