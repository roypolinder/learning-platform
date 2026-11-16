use std::collections::BTreeMap;
use std::sync::Arc;

use lp_core::{Extension, ExtensionManifest, StepTypeSpec};

use crate::builtin;

#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("extension `{0}` is already registered")]
    DuplicateExtension(String),
    #[error("step type `{step_type}` is provided by both `{first}` and `{second}`")]
    DuplicateStepType {
        step_type: String,
        first: String,
        second: String,
    },
}

#[derive(Clone, Default)]
pub struct ExtensionRegistry {
    extensions: BTreeMap<&'static str, Arc<dyn Extension>>,
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, extension: Arc<dyn Extension>) -> Result<(), RegistryError> {
        let id = extension.manifest().id;
        if self.extensions.contains_key(id) {
            return Err(RegistryError::DuplicateExtension(id.to_owned()));
        }
        self.extensions.insert(id, extension);
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&Arc<dyn Extension>> {
        self.extensions.get(id)
    }

    pub fn manifests(&self) -> impl Iterator<Item = &ExtensionManifest> {
        self.extensions
            .values()
            .map(|extension| extension.manifest())
    }

    pub fn len(&self) -> usize {
        self.extensions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.extensions.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct StepTypeEntry {
    pub spec: StepTypeSpec,
    /// Extension that contributed the type; `None` for engine built-ins.
    pub provider: Option<&'static str>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct StepTypeRegistry {
    entries: BTreeMap<&'static str, StepTypeEntry>,
}

impl StepTypeRegistry {
    pub fn build(extensions: &ExtensionRegistry) -> Result<Self, RegistryError> {
        let mut registry = Self::default();
        for spec in builtin::specs() {
            registry.insert(spec.clone(), None)?;
        }
        for manifest in extensions.manifests() {
            for spec in &manifest.step_types {
                registry.insert(spec.clone(), Some(manifest.id))?;
            }
        }
        Ok(registry)
    }

    fn insert(
        &mut self,
        spec: StepTypeSpec,
        provider: Option<&'static str>,
    ) -> Result<(), RegistryError> {
        if let Some(existing) = self.entries.get(spec.name) {
            return Err(RegistryError::DuplicateStepType {
                step_type: spec.name.to_owned(),
                first: existing.provider.unwrap_or("engine").to_owned(),
                second: provider.unwrap_or("engine").to_owned(),
            });
        }
        self.entries
            .insert(spec.name, StepTypeEntry { spec, provider });
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&StepTypeEntry> {
        self.entries.get(name)
    }

    pub fn entries(&self) -> impl Iterator<Item = &StepTypeEntry> {
        self.entries.values()
    }
}
