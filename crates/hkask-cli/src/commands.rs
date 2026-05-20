//! CLI commands implementation
//!
//! This module contains the actual command handlers.

use hkask_templates::{
    EnergyCalibrator, ProcessManifest, RegistryEntry, RegistryIndex, SqliteRegistry, TemplateError,
};
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
    _registry: &dyn RegistryIndex,
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

/// Energy cap calibration command
///
/// Analyzes manifest energy budgets and provides calibration recommendations
/// based on actual consumption patterns.
#[allow(dead_code)]
pub fn calibrate_energy_caps(
    manifest_path: &Path,
) -> Result<EnergyCalibrationReport, TemplateError> {
    use serde_yaml::Value as YamlValue;
    use std::fs;

    let content = fs::read_to_string(manifest_path)
        .map_err(|e| TemplateError::Manifest(format!("Failed to read manifest: {}", e)))?;

    let yaml: YamlValue = serde_yaml::from_str(&content)
        .map_err(|e| TemplateError::Manifest(format!("Failed to parse YAML: {}", e)))?;

    // Extract energy configuration
    let energy_config = yaml
        .get("energy")
        .ok_or_else(|| TemplateError::Manifest("No energy configuration found".to_string()))?;

    let cap = energy_config
        .get("cap")
        .and_then(|v| v.as_u64())
        .unwrap_or(10000);

    let cost_per_token = energy_config
        .get("cost_per_token")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.25);

    let alert_threshold = energy_config
        .get("alert_threshold")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.8);

    // Create energy account for analysis
    let mut account = EnergyAccount::new(
        manifest_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown"),
        cap,
    );
    account.budget.cost_per_token = cost_per_token;
    account.budget.alert_threshold = alert_threshold;

    // Analyze steps for energy requirements
    let steps = yaml
        .get("steps")
        .and_then(|v| v.as_sequence())
        .map(|s| s.len())
        .unwrap_or(0);

    let estimated_cost_per_step = 500; // Default estimate
    let total_estimated_cost = (steps * estimated_cost_per_step) as u64;

    // Calculate recommended cap (20% buffer above estimated)
    let recommended_cap = (total_estimated_cost as f64 * 1.2) as u64;
    let cap_utilization = if cap > 0 {
        (total_estimated_cost as f64 / cap as f64) * 100.0
    } else {
        0.0
    };

    let recommendation = if cap_utilization < 50.0 {
        "Cap is oversized - consider reducing"
    } else if cap_utilization > 90.0 {
        "Cap is too tight - consider increasing"
    } else {
        "Cap is well-calibrated"
    };

    Ok(EnergyCalibrationReport {
        manifest_id: manifest_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string(),
        current_cap: cap,
        recommended_cap,
        cap_utilization,
        estimated_cost: total_estimated_cost,
        steps_count: steps,
        cost_per_token,
        alert_threshold,
        recommendation: recommendation.to_string(),
    })
}

/// Energy calibration report
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[allow(dead_code)]
pub struct EnergyCalibrationReport {
    pub manifest_id: String,
    pub current_cap: u64,
    pub recommended_cap: u64,
    pub cap_utilization: f64,
    pub estimated_cost: u64,
    pub steps_count: usize,
    pub cost_per_token: f64,
    pub alert_threshold: f64,
    pub recommendation: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_templates() {
        // Test would require a mock registry
    }
}
