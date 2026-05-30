//! Russell → hKask Semantic Mapper — Simplified
//!
//! Generic YAML mapper that applies russell-mapping.yaml configuration.
//! Rust is the loom. YAML is the thread. Russell is the legacy library.
//! ℏKask v0.21.2 — A Minimal Viable Container for Agents

use hkask_types::lexicon::TemplateType;
use hkask_types::{Phase, Span};
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
    pub authored: Option<String>,
    #[serde(default)]
    pub applies_when: Vec<String>,
    #[serde(default)]
    pub symptoms: Vec<String>,
    #[serde(default)]
    pub probes: Vec<Value>,
    #[serde(default)]
    pub interventions: Vec<Value>,
    #[serde(default)]
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
    pub energy_budget: Option<EnergyBudget>,
    #[serde(default)]
    pub cns_spans: Option<CnsSpans>,
    #[serde(default)]
    pub output: Option<OutputConfig>,
    #[serde(default)]
    pub dry_run: bool,
}

impl RussellMappingConfig {
    pub fn load_from_yaml(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| MapperError::IoError {
            path: PathBuf::from(path),
            source: e,
        })?;
        serde_yaml::from_str(&content).map_err(MapperError::YamlParse)
    }

    pub fn defaults() -> Self {
        Self {
            mapping: MappingMeta {
                version: "0.21.2".to_string(),
                description: "Russell to hKask mapping".to_string(),
                functional_role: None,
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
                russell_authored: None,
                russell_symptoms: FieldMapping {
                    to: "template_description".to_string(),
                    transform: "join_with_newlines".to_string(),
                },
                russell_applies_when: None,
                russell_probes: None,
                russell_interventions: None,
                russell_safety: None,
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
            energy_budget: None,
            cns_spans: None,
            output: None,
            dry_run: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingMeta {
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub functional_role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMappings {
    pub russell_id: FieldMapping,
    pub russell_version: FieldMapping,
    #[serde(default)]
    pub russell_authored: Option<FieldMapping>,
    pub russell_symptoms: FieldMapping,
    #[serde(default)]
    pub russell_applies_when: Option<FieldMapping>,
    #[serde(default)]
    pub russell_probes: Option<FieldMapping>,
    #[serde(default)]
    pub russell_interventions: Option<FieldMapping>,
    #[serde(default)]
    pub russell_safety: Option<FieldMapping>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyBudget {
    pub base_cost: u64,
    pub per_probe_cost: u64,
    pub per_intervention_cost: u64,
    #[serde(default)]
    pub risk_multiplier: std::collections::HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CnsSpans {
    #[serde(default = "default_true")]
    pub emit_on_mapping: bool,
    #[serde(default = "default_cns_namespace")]
    pub span_namespace: String,
    #[serde(default)]
    pub phases: Vec<String>,
}

fn default_true() -> bool {
    true
}

fn default_cns_namespace() -> String {
    "cns.template.russell_mapping".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    #[serde(default = "default_output_format")]
    pub format: String,
    #[serde(default = "default_true")]
    pub include_provenance: bool,
    #[serde(default)]
    pub provenance_fields: Vec<String>,
}

fn default_output_format() -> String {
    "yaml".to_string()
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
    config: RussellMappingConfig,
    cns: hkask_cns::SpanEmitter,
}

impl Default for RussellMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl RussellMapper {
    pub fn new() -> Self {
        Self {
            config: RussellMappingConfig::defaults(),
            cns: hkask_cns::SpanEmitter::default(),
        }
    }

    pub fn with_config(config: RussellMappingConfig) -> Self {
        Self {
            config,
            cns: hkask_cns::SpanEmitter::default(),
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

        if self.should_emit_cns() {
            self.cns.emit_with_phase(
                Span::pipeline("russell_manifest_analyzed"),
                Phase::Observe,
                serde_json::json!({
                    "manifest_id": manifest.id,
                    "version": manifest.version,
                    "source": yaml_path.to_string_lossy(),
                }),
            );
        }

        Ok(manifest)
    }

    /// Map Russell manifest to hKask template
    pub fn map_to_hkask(&self, russell: &RussellSkillManifest) -> MappedTemplate {
        let hkask_id = transform_id(&russell.id, &self.config.id_transformation);
        let template_type = infer_template_type(russell, &self.config.template_type_inference);
        let model_tier = select_model_tier(russell, &self.config.model_tier_selection);
        let description = russell.symptoms.join("\n");
        let energy_cap = calculate_energy_budget(russell, self.config.energy_budget.as_ref());

        if self.should_emit_cns() {
            self.cns.emit_with_phase(
                Span::pipeline("russell_mapping_complete"),
                Phase::Observe,
                serde_json::json!({
                    "source_id": russell.id,
                    "mapped_id": hkask_id,
                    "template_type": format!("{:?}", template_type),
                    "model_tier": model_tier,
                    "energy_cap": energy_cap,
                    "phase": "mapped",
                }),
            );
        }

        MappedTemplate {
            id: hkask_id,
            template_type,
            description,
            model_tier,
            energy_cap,
        }
    }

    fn should_emit_cns(&self) -> bool {
        self.config
            .cns_spans
            .as_ref()
            .map(|s| s.emit_on_mapping)
            .unwrap_or(true)
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
fn calculate_energy_budget(
    russell: &RussellSkillManifest,
    budget_cfg: Option<&EnergyBudget>,
) -> u64 {
    let base_cost: u64 = budget_cfg.map(|b| b.base_cost).unwrap_or(1000);
    let per_probe_cost: u64 = budget_cfg.map(|b| b.per_probe_cost).unwrap_or(200);
    let per_intervention_cost: u64 = budget_cfg.map(|b| b.per_intervention_cost).unwrap_or(500);

    base_cost
        + (russell.probes.len() as u64 * per_probe_cost)
        + (russell.interventions.len() as u64 * per_intervention_cost)
}
