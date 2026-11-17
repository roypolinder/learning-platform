//! Contracts shared by the course engine, extensions and the API.
//!
//! Anything in this crate is part of the extension API surface; changes here
//! ripple into every extension, so keep it small and stable.

pub mod diagram;
pub mod extension;
pub mod outcome;
pub mod status;
pub mod step_type;

pub use diagram::{Diagram, DiagramEdge, DiagramNode, NodeKind, SlotRule};
pub use extension::{
    Capability, Extension, ExtensionError, ExtensionEvent, ExtensionManifest, StepContext,
    Submission, SubmissionSource, parse_config,
};
pub use outcome::{CheckResult, OutcomeStatus, ValidationOutcome};
pub use status::{SessionStatus, StepStatus, TransitionError, UnknownStatus};
pub use step_type::{CompletionStrategy, StepCategory, StepTypeSpec};
