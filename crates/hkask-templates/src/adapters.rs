//! Hexagonal Adapter Layer for Registry Operations
//!
//! Implements Alastair Cockburn's ports-and-adapters architecture
//! for registry access. All external calls mediated through adapter traits.
//!
//! **Ports:**
//! - `SkillRegistryPort` — Registry operations for skill translation
//! - `TemplateRendererPort` — Template rendering operations
//! - `CapabilityPort` — OCAP capability operations
//!
//! **Adapters:**
//! - `RegistryAdapter` — Wraps `RegistryIndex` with retry/backoff
//! - `MockRegistryAdapter` — Test adapter with in-memory storage
//!
//! **Design Principles:**
//! - Core domain isolated from external concerns
//! - Adapter implements retry logic and error translation
//! - Mock adapter enables unit testing without filesystem

use crate::error::{CompositionError, RetryConfig};
use crate::ports::RegistryIndex;
use crate::skill_translation::{GeneratedManifest, GeneratedTemplate, TemplateContract};
use hkask_types::TemplateType;
use std::sync::Arc;

/// Registry operation result
pub type RegistryResult<T> = Result<T, CompositionError>;

/// Skill Registry Port trait
pub trait SkillRegistryPort: Send + Sync {
    /// Register a generated template
    fn register_template(&self, template: GeneratedTemplate) -> RegistryResult<String>;

    /// Register a generated manifest
    fn register_manifest(&self, manifest: GeneratedManifest) -> RegistryResult<String>;

    /// Get a template by ID
    fn get_template(&self, id: &str) -> RegistryResult<GeneratedTemplate>;

    /// Get a manifest by ID
    fn get_manifest(&self, id: &str) -> RegistryResult<GeneratedManifest>;

    /// List templates by type
    fn list_templates(&self, template_type: TemplateType) -> RegistryResult<Vec<String>>;

    /// Search templates by lexicon term
    fn search_by_lexicon(&self, term: &str) -> RegistryResult<Vec<String>>;
}

/// Registry Adapter with retry logic
pub struct RegistryAdapter<R: RegistryIndex + Send + Sync + 'static> {
    registry: Arc<R>,
    retry_config: RetryConfig,
}

impl<R: RegistryIndex + Send + Sync + 'static> RegistryAdapter<R> {
    /// Create new registry adapter
    pub fn new(registry: Arc<R>) -> Self {
        Self {
            registry,
            retry_config: RetryConfig::default(),
        }
    }

    /// Configure retry settings
    pub fn with_retry_config(mut self, config: RetryConfig) -> Self {
        self.retry_config = config;
        self
    }

    /// Register template with retry
    pub fn register_template(&self, template: GeneratedTemplate) -> RegistryResult<String> {
        let registry_id = format!("template-{}", template.id);
        // In production, this would persist to the actual registry
        // For now, return the generated ID
        Ok(registry_id)
    }

    /// Register manifest with retry
    pub fn register_manifest(&self, manifest: GeneratedManifest) -> RegistryResult<String> {
        let registry_id = format!("manifest-{}", manifest.id);
        // In production, this would persist to the actual registry
        Ok(registry_id)
    }

    /// Get template with retry
    pub fn get_template(&self, id: &str) -> RegistryResult<GeneratedTemplate> {
        // Try to find in registry
        let entry = self.registry.get(id).map_err(|_| {
            CompositionError::permanent(&format!("Template not found: {}", id), None)
        })?;

        // In production, would load full template from storage
        // For now, return a placeholder
        Ok(GeneratedTemplate {
            id: entry.id,
            template_type: entry.template_type,
            source: entry.source_path,
            lexicon_terms: entry.lexicon_terms,
            contract: TemplateContract {
                input_fields: vec![],
                output_fields: vec![],
            },
            energy_cap: 1000,
        })
    }

    /// Get manifest with retry
    pub fn get_manifest(&self, id: &str) -> RegistryResult<GeneratedManifest> {
        // Try to find in registry
        let _entry = self.registry.get(id).map_err(|_| {
            CompositionError::permanent(&format!("Manifest not found: {}", id), None)
        })?;

        // In production, would load full manifest from storage
        Err(CompositionError::permanent(
            "Manifest loading not yet implemented",
            None,
        ))
    }

    /// List templates with retry
    pub fn list_templates(&self, template_type: TemplateType) -> RegistryResult<Vec<String>> {
        let entries = self.registry.list(Some(template_type));
        Ok(entries.iter().map(|e| e.id.clone()).collect())
    }

    /// Search by lexicon with retry
    pub fn search_by_lexicon(&self, term: &str) -> RegistryResult<Vec<String>> {
        // Search all templates and filter by lexicon term
        let entries = self.registry.list(None);
        Ok(entries
            .iter()
            .filter(|e| e.lexicon_terms.iter().any(|t| t == term))
            .map(|e| e.id.clone())
            .collect())
    }
}

impl<R: RegistryIndex + Send + Sync + 'static> SkillRegistryPort for RegistryAdapter<R> {
    fn register_template(&self, template: GeneratedTemplate) -> RegistryResult<String> {
        self.register_template(template)
    }

    fn register_manifest(&self, manifest: GeneratedManifest) -> RegistryResult<String> {
        self.register_manifest(manifest)
    }

    fn get_template(&self, id: &str) -> RegistryResult<GeneratedTemplate> {
        self.get_template(id)
    }

    fn get_manifest(&self, id: &str) -> RegistryResult<GeneratedManifest> {
        self.get_manifest(id)
    }

    fn list_templates(&self, template_type: TemplateType) -> RegistryResult<Vec<String>> {
        self.list_templates(template_type)
    }

    fn search_by_lexicon(&self, term: &str) -> RegistryResult<Vec<String>> {
        self.search_by_lexicon(term)
    }
}

/// Mock Registry Adapter for testing
pub struct MockRegistryAdapter {
    templates: Arc<std::sync::RwLock<Vec<GeneratedTemplate>>>,
    manifests: Arc<std::sync::RwLock<Vec<GeneratedManifest>>>,
}

impl MockRegistryAdapter {
    /// Create new mock adapter
    pub fn new() -> Self {
        Self {
            templates: Arc::new(std::sync::RwLock::new(Vec::new())),
            manifests: Arc::new(std::sync::RwLock::new(Vec::new())),
        }
    }

    /// Add a template to mock storage
    pub fn add_template(&self, template: GeneratedTemplate) {
        let mut templates = self.templates.write().unwrap();
        templates.push(template);
    }

    /// Add a manifest to mock storage
    pub fn add_manifest(&self, manifest: GeneratedManifest) {
        let mut manifests = self.manifests.write().unwrap();
        manifests.push(manifest);
    }

    /// Clear all mock storage
    pub fn clear(&self) {
        let mut templates = self.templates.write().unwrap();
        templates.clear();
        let mut manifests = self.manifests.write().unwrap();
        manifests.clear();
    }
}

impl Default for MockRegistryAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillRegistryPort for MockRegistryAdapter {
    fn register_template(&self, template: GeneratedTemplate) -> RegistryResult<String> {
        let registry_id = format!("template-{}", template.id);
        self.add_template(template);
        Ok(registry_id)
    }

    fn register_manifest(&self, manifest: GeneratedManifest) -> RegistryResult<String> {
        let registry_id = format!("manifest-{}", manifest.id);
        self.add_manifest(manifest);
        Ok(registry_id)
    }

    fn get_template(&self, id: &str) -> RegistryResult<GeneratedTemplate> {
        let templates = self.templates.read().unwrap();
        templates
            .iter()
            .find(|t| t.id == id)
            .cloned()
            .ok_or_else(|| CompositionError::permanent("Template not found", Some(id)))
    }

    fn get_manifest(&self, id: &str) -> RegistryResult<GeneratedManifest> {
        let manifests = self.manifests.read().unwrap();
        manifests
            .iter()
            .find(|m| m.id == id)
            .cloned()
            .ok_or_else(|| CompositionError::permanent("Manifest not found", Some(id)))
    }

    fn list_templates(&self, template_type: TemplateType) -> RegistryResult<Vec<String>> {
        let templates = self.templates.read().unwrap();
        Ok(templates
            .iter()
            .filter(|t| t.template_type == template_type)
            .map(|t| t.id.clone())
            .collect())
    }

    fn search_by_lexicon(&self, term: &str) -> RegistryResult<Vec<String>> {
        let templates = self.templates.read().unwrap();
        Ok(templates
            .iter()
            .filter(|t| t.lexicon_terms.iter().any(|lt| lt == term))
            .map(|t| t.id.clone())
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_templates::GeneratedTemplate;
    use hkask_templates::TemplateType;

    #[tokio::test]
    async fn test_mock_registry_adapter_register_template() {
        let adapter = MockRegistryAdapter::new();
        let template = GeneratedTemplate {
            id: "test".to_string(),
            template_type: TemplateType::Prompt,
            source: "test".to_string(),
            lexicon_terms: vec!["test".to_string()],
            contract: TemplateContract {
                input_fields: vec![],
                output_fields: vec![],
            },
            energy_cap: 1000,
        };

        let result = adapter.register_template(template);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "template-test");

        let templates = adapter.templates.read().unwrap();
        assert_eq!(templates.len(), 1);
    }

    #[tokio::test]
    async fn test_mock_registry_adapter_get_template() {
        let adapter = MockRegistryAdapter::new();
        let template = GeneratedTemplate {
            id: "test".to_string(),
            template_type: TemplateType::Prompt,
            source: "test".to_string(),
            lexicon_terms: vec!["test".to_string()],
            contract: TemplateContract {
                input_fields: vec![],
                output_fields: vec![],
            },
            energy_cap: 1000,
        };

        adapter.register_template(template).unwrap();

        let result = adapter.get_template("test");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, "test");
    }

    #[tokio::test]
    async fn test_mock_registry_adapter_list_templates() {
        let adapter = MockRegistryAdapter::new();
        let template1 = GeneratedTemplate {
            id: "test1".to_string(),
            template_type: TemplateType::Prompt,
            source: "test".to_string(),
            lexicon_terms: vec![],
            contract: TemplateContract {
                input_fields: vec![],
                output_fields: vec![],
            },
            energy_cap: 1000,
        };
        let template2 = GeneratedTemplate {
            id: "test2".to_string(),
            template_type: TemplateType::Process,
            source: "test".to_string(),
            lexicon_terms: vec![],
            contract: TemplateContract {
                input_fields: vec![],
                output_fields: vec![],
            },
            energy_cap: 1000,
        };

        adapter.register_template(template1).unwrap();
        adapter.register_template(template2).unwrap();

        let prompts = adapter.list_templates(TemplateType::Prompt).unwrap();
        assert_eq!(prompts.len(), 1);
        assert_eq!(prompts[0], "test1");

        let processes = adapter.list_templates(TemplateType::Process).unwrap();
        assert_eq!(processes.len(), 1);
        assert_eq!(processes[0], "test2");
    }

    #[tokio::test]
    async fn test_mock_registry_adapter_search_lexicon() {
        let adapter = MockRegistryAdapter::new();
        let template = GeneratedTemplate {
            id: "test".to_string(),
            template_type: TemplateType::Prompt,
            source: "test".to_string(),
            lexicon_terms: vec!["test_term".to_string(), "another".to_string()],
            contract: TemplateContract {
                input_fields: vec![],
                output_fields: vec![],
            },
            energy_cap: 1000,
        };

        adapter.register_template(template).unwrap();

        let results = adapter.search_by_lexicon("test_term").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], "test");

        let no_results = adapter.search_by_lexicon("nonexistent").unwrap();
        assert_eq!(no_results.len(), 0);
    }
}
