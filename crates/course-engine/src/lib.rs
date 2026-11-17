//! The learning engine: turns declarative course files into executable courses and
//! drives learner progress through a step state machine.
//!
//! The engine knows step *categories* and *completion strategies*, never what a
//! particular integration does. Integration behaviour lives in extensions that are
//! registered through [`ExtensionRegistry`].

mod builtin;
pub mod catalog;
mod compiler;
pub mod course;
pub mod definition;
mod error;
pub mod localization;
pub mod personalize;
pub mod registry;
pub mod runtime;
pub mod theme;
pub mod view;

pub use catalog::{
    CatalogError, check_renders, load_directory, parse_definition, render_definition,
};
pub use course::{Course, CourseStep, SectionOutline};
pub use definition::{CourseDefinition, GlossaryEntry};
pub use error::{CourseIssue, CourseValidationError, EngineError};
pub use localization::Locale;
pub use personalize::Audience;
pub use registry::{ExtensionRegistry, RegistryError, StepTypeEntry};
pub use runtime::{
    Advance, CourseEngine, EngineContext, Progress, SessionIdentity, StepState, StepTransition,
    TransitionReason,
};
pub use theme::{Theme, ThemeCatalog, ThemeSummary, load_themes};
