//! CLI commands implementation
//!
//! This module contains the actual command handlers.

use hkask_templates::{RegistryEntry, RegistryIndex, SqliteRegistry, TemplateError, ManifestExecutor, ProcessManifest};
use hkask_types::TemplateType;
use serde_json::Value;
use std::path::Path;

/// Template list command
pub fn list_templates(
    registry: &dyn RegistryIndex,
    template_type: Option<TemplateType>,
) -> Vec<RegistryEntry> {
    registry.list(template_type)
}

/// Register template command
pub fn register_template(
    registry: &mut SqliteRegistry,
    id: String,
    template_type: TemplateType,
    source_path: String,
    lexicon_terms: Vec<String>,
    description: String,
) -> Result<(), TemplateError> {
    let entry = RegistryEntry {
        id,
        template_type,
        lexicon_terms,
        description,
        source_path,
    };

    registry.register(entry, None)
}

/// Get template command
pub fn get_template(
    registry: &dyn RegistryIndex,
    id: &str,
) -> Result<RegistryEntry, TemplateError> {
    registry.get(id)
}

/// Search templates by lexicon
pub fn search_templates(registry: &SqliteRegistry, term: &str) -> Vec<RegistryEntry> {
    registry.search_by_lexicon(term)
}

/// Render template command
pub fn render_template(
    registry: &dyn RegistryIndex,
    template_id: &str,
    bindings: Value,
) -> Result<String, TemplateError> {
    // Get template entry
    let entry = registry.get(template_id)?;
    
    // Read template source
    let source = std::fs::read_to_string(&entry.source_path)
        .map_err(|e| TemplateError::Render(format!("Failed to read template: {}", e)))?;
    
    // For now, return source with bindings info
    // Full rendering requires minijinja integration
    Ok(format!(
        "Template: {}\nBindings: {}\nSource: {}",
        template_id,
        serde_json::to_string_pretty(&bindings).unwrap_or_default(),
        source
    ))
}

/// Execute manifest command
pub fn execute_manifest(
    registry: &dyn RegistryIndex,
    manifest_id: &str,
    input: Value,
) -> Result<Value, TemplateError> {
    // Load manifest from YAML
    let manifest_path = format!("registry/manifests/{}.yaml", manifest_id);
    let manifest = ProcessManifest::load_from_yaml(Path::new(&manifest_path))
        .map_err(|e| TemplateError::Manifest(format!("Failed to load manifest: {}", e)))?;
    
    // For now, return manifest info
    // Full execution requires ManifestExecutor integration
    Ok(serde_json::json!({
        "manifest_id": manifest.id,
        "name": manifest.name,
        "description": manifest.description,
        "steps": manifest.steps.len(),
        "input": input
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_templates() {
        // Test would require a mock registry
    }
}
