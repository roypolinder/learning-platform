//! Built-in extensions. Each module is self-contained and only depends on the
//! `lp-core` contract, so adding one never touches the engine.

pub mod client_run;
mod evidence;
pub mod filesystem;
pub mod git;
pub mod http;
pub mod mcp;
pub mod quiz;
pub mod rules;
pub mod service;
mod support;

use std::path::PathBuf;
use std::sync::Arc;

use lp_core::Extension;

#[derive(Debug, Clone, Default)]
pub struct ExtensionOptions {
    /// Fixtures that service checks send to learner services (e.g. test images).
    pub assets_dir: Option<PathBuf>,
}

pub fn builtin_extensions(options: &ExtensionOptions) -> Vec<Arc<dyn Extension>> {
    vec![
        Arc::new(quiz::QuizExtension::new()),
        Arc::new(http::HttpExtension::new()),
        Arc::new(filesystem::FilesystemExtension::new()),
        Arc::new(git::GitExtension::new()),
        Arc::new(mcp::McpExtension::new()),
        Arc::new(service::ServiceExtension::new(service::ServiceOptions {
            assets_dir: options.assets_dir.clone(),
        })),
    ]
}
