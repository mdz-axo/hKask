//! Template Resolver — Maps template IDs to paths via registry
//!
//! Decouples manifests from filesystem paths, enabling registry abstraction.
//! Per architecture v0.21.0: Loose coupling via indirection.
//! Per Planck minimalism: No cache — registry lookups are O(1) SQLite queries.

use crate::ports::{RegistryIndex, Result};

/// Template resolver — direct registry lookup without caching
///
/// Following Planck's constant minimalism: remove state when function can be computed directly.
/// Registry lookups are O(1) SQLite queries — caching adds complexity without measurable benefit.
pub struct TemplateResolver<R> {
    registry: R,
}

impl<R: RegistryIndex> TemplateResolver<R> {
    /// Create new resolver
    pub fn new(registry: R) -> Self {
        Self { registry }
    }

    /// Resolve template ID to path via direct registry lookup
    pub fn resolve(&self, template_id: &str) -> Result<String> {
        // Direct registry lookup — O(1) SQLite query
        let entry = self.registry.get(template_id)?;
        Ok(entry.source_path)
    }
}


    impl MockRegistry {
        fn new() -> Self {
            Self {
                entries: HashMap::new(),
            }
        }

        fn add(&mut self, entry: RegistryEntry) {
            self.entries.insert(entry.id.clone(), entry);
        }
    }

    impl RegistryIndex for MockRegistry {
        fn get(&self, id: &str) -> Result<RegistryEntry> {
            self.entries
                .get(id)
                .cloned()
                .ok_or_else(|| TemplateError::NotFound(id.to_string()))
        }

        fn list(&self, _template_type: Option<TemplateType>) -> Vec<RegistryEntry> {
            self.entries.values().cloned().collect()
        }

        fn bootstrap_manifest(&self) -> Option<crate::ports::ProcessManifest> {
            None
        }
    }



}
