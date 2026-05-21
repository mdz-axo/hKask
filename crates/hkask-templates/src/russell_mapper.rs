//! Russell → hKask Semantic Mapper — Simplified
//!
//! Generic YAML mapper that applies russell-mapping.yaml configuration.
//! Rust is the loom. YAML is the thread. Russell is the legacy library.
//! ℏKask v0.21.2 — Planck's Constant of Agent Systems

use hkask_types::lexicon::TemplateType;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MapperError {
    #[error("Failed to parse Russell YAML: {0}")]
    YamlParse(serde_yaml::Error),
    #[error("Failed to read file {path}")]
    IoError {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Invalid Russell manifest: missing required field '{field}'")]
    MissingField { field: String },
}

pub type Result<T> = std::result::Result<T, MapperError>;

/// Russell skill manifest (minimal structure)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RussellSkillManifest {
    pub id: String,
    pub version: String,
    #[serde(default)]
    pub symptoms: Vec<String>,
    #[serde(default)]
    pub probes: Vec<Value>,
    #[serde(default)]
    pub interventions: Vec<Value>,
    pub safety: Value,
}

/// Mapping configuration (loaded from registry/manifests/russell-mapping.yaml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RussellMappingConfig {
    pub mapping: MappingMeta,
    pub field_mappings: FieldMappings,
    pub id_transformation: IdTransformation,
    pub template_type_inference: TemplateTypeInference,
    pub model_tier_selection: ModelTierSelection,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingMeta {
    pub version: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMappings {
    pub russell_id: FieldMapping,
    pub russell_version: FieldMapping,
    pub russell_symptoms: FieldMapping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMapping {
    pub to: String,
    pub transform: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdTransformation {
    pub prefix: String,
    pub preserve_suffix: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateTypeInference {
    pub rules: Vec<TypeRule>,
    pub default: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeRule {
    #[serde(rename = "if")]
    pub condition: String,
    pub then: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelTierSelection {
    pub rules: Vec<TierRule>,
    pub default: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierRule {
    #[serde(rename = "if")]
    pub condition: String,
    pub then: String,
}

/// Mapped template configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappedTemplate {
    pub id: String,
    pub template_type: TemplateType,
    pub description: String,
    pub model_tier: String,
    pub energy_cap: u64,
}

/// Russell mapper — generic YAML processor
pub struct RussellMapper {
    #[allow(dead_code)]
    config: RussellMappingConfig,
}

impl Default for RussellMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl RussellMapper {
    pub fn new() -> Self {
        Self {
            config: RussellMappingConfig {
                mapping: MappingMeta {
                    version: "0.21.2".to_string(),
                    description: "Russell to hKask mapping".to_string(),
                },
                field_mappings: FieldMappings {
                    russell_id: FieldMapping {
                        to: "hKask_id".to_string(),
                        transform: "prefix_with_skill_hkask".to_string(),
                    },
                    russell_version: FieldMapping {
                        to: "template_version".to_string(),
                        transform: "passthrough".to_string(),
                    },
                    russell_symptoms: FieldMapping {
                        to: "template_description".to_string(),
                        transform: "join_with_newlines".to_string(),
                    },
                },
                id_transformation: IdTransformation {
                    prefix: "skill/hkask/".to_string(),
                    preserve_suffix: true,
                },
                template_type_inference: TemplateTypeInference {
                    rules: vec![],
                    default: "Process".to_string(),
                },
                model_tier_selection: ModelTierSelection {
                    rules: vec![],
                    default: "balanced".to_string(),
                },
                dry_run: false,
            },
        }
    }

    /// Analyze Russell skill manifest
    pub fn analyze_skill_manifest(&self, yaml_path: &Path) -> Result<RussellSkillManifest> {
        let content = std::fs::read_to_string(yaml_path).map_err(|e| MapperError::IoError {
            path: yaml_path.to_path_buf(),
            source: e,
        })?;

        let manifest: RussellSkillManifest =
            serde_yaml::from_str(&content).map_err(MapperError::YamlParse)?;

        if manifest.id.is_empty() {
            return Err(MapperError::MissingField {
                field: "id".to_string(),
            });
        }

        Ok(manifest)
    }

    /// Map Russell manifest to hKask template
    pub fn map_to_hkask(&self, russell: &RussellSkillManifest) -> MappedTemplate {
        let hkask_id = transform_id(&russell.id, &self.config.id_transformation);
        let template_type = infer_template_type(russell, &self.config.template_type_inference);
        let model_tier = select_model_tier(russell, &self.config.model_tier_selection);
        let description = russell.symptoms.join("\n");
        let energy_cap = calculate_energy_budget(russell);

        MappedTemplate {
            id: hkask_id,
            template_type,
            description,
            model_tier,
            energy_cap,
        }
    }
}

/// Transform Russell ID to hKask ID
fn transform_id(russell_id: &str, config: &IdTransformation) -> String {
    let suffix = russell_id
        .strip_prefix("skill/russell/")
        .unwrap_or(russell_id);
    format!("{}{}", config.prefix, suffix)
}

/// Infer template type from Russell manifest
fn infer_template_type(
    russell: &RussellSkillManifest,
    _config: &TemplateTypeInference,
) -> TemplateType {
    let probe_count = russell.probes.len();
    let intervention_count = russell.interventions.len();

    if probe_count > 0 && intervention_count > 0 {
        TemplateType::Process
    } else if probe_count > 0 {
        TemplateType::Prompt
    } else {
        TemplateType::Cognition
    }
}

/// Select model tier based on Russell manifest
fn select_model_tier(russell: &RussellSkillManifest, config: &ModelTierSelection) -> String {
    if russell.symptoms.len() <= 3 {
        "fast_local".to_string()
    } else {
        config.default.clone()
    }
}

/// Calculate energy budget for mapped template
fn calculate_energy_budget(russell: &RussellSkillManifest) -> u64 {
    let base_cost: u64 = 1000;
    let per_probe_cost: u64 = 200;
    let per_intervention_cost: u64 = 500;

    base_cost
        + (russell.probes.len() as u64 * per_probe_cost)
        + (russell.interventions.len() as u64 * per_intervention_cost)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_transformation() {
        let config = IdTransformation {
            prefix: "skill/hkask/".to_string(),
            preserve_suffix: true,
        };
        assert_eq!(
            transform_id("skill/russell/semantic", &config),
            "skill/hkask/semantic"
        );
    }

    #[test]
    fn test_template_type_inference() {
        let russell = RussellSkillManifest {
            id: "test".to_string(),
            version: "1.0".to_string(),
            symptoms: vec![],
            probes: vec![Value::String("test".to_string())],
            interventions: vec![Value::String("test".to_string())],
            safety: Value::Null,
        };
        let config = TemplateTypeInference {
            rules: vec![],
            default: "Process".to_string(),
        };
        assert_eq!(
            infer_template_type(&russell, &config),
            TemplateType::Process
        );
    }

    #[test]
    fn test_energy_budget_calculation() {
        let russell = RussellSkillManifest {
            id: "test".to_string(),
            version: "1.0".to_string(),
            symptoms: vec![],
            probes: vec![Value::String("test".to_string())],
            interventions: vec![Value::String("test".to_string())],
            safety: Value::Null,
        };
        let energy = calculate_energy_budget(&russell);
        assert_eq!(energy, 1700); // 1000 + 200 + 500
    }
}
