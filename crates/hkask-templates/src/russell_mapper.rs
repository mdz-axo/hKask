//! Russell → hKask Semantic Mapper
//!
//! Transforms Russell skill manifests and prompt templates into hKask registry entries.
//! Rust is the loom. YAML/Jinja2 is the thread. Russell is the legacy library.

use crate::ports::{CompositionTemplate, ProcessManifest, TemplateContract};
use crate::provenance::ProvenanceManager;
use chrono::Utc;
use hkask_types::TemplateType;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::debug;

#[derive(Error, Debug)]
pub enum MapperError {
    #[error("Failed to parse Russell YAML: {0}")]
    YamlParse(#[from] serde_yaml::Error),

    #[error("Failed to read file {path}: {source}")]
    IoError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Invalid Russell manifest: missing required field '{field}'")]
    MissingField { field: String },
}

pub type Result<T> = std::result::Result<T, MapperError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RussellSkillManifest {
    pub id: String,
    pub version: String,
    #[serde(default)]
    pub authored: String,
    #[serde(default)]
    pub min_harness_version: String,
    #[serde(default)]
    pub symptoms: Vec<String>,
    #[serde(default)]
    pub applies_when: Vec<AppliesWhenCondition>,
    #[serde(default)]
    pub probes: Vec<RussellProbe>,
    #[serde(default)]
    pub interventions: Vec<RussellIntervention>,
    pub safety: RussellSafety,
    #[serde(default)]
    pub references: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AppliesWhenCondition {
    Simple(String),
    Structured {
        #[serde(default)]
        os_family: Option<String>,
        #[serde(default)]
        os_version: Option<String>,
        #[serde(default)]
        environment: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RussellProbe {
    pub id: String,
    #[serde(default)]
    pub cmd: String,
    #[serde(default)]
    pub capture: String,
    #[serde(default)]
    pub timeout: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RussellIntervention {
    pub id: String,
    #[serde(default)]
    pub cmd: String,
    #[serde(default)]
    pub risk: String,
    #[serde(default)]
    pub idempotent: bool,
    #[serde(default)]
    pub rollback: Option<String>,
    #[serde(default)]
    pub timeout: String,
    #[serde(default)]
    pub needs_sudo: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RussellSafety {
    #[serde(default)]
    pub max_auto_risk: String,
    #[serde(default)]
    pub require_human_for: Vec<String>,
    #[serde(default)]
    pub allowed_env_keys: Vec<String>,
    #[serde(default)]
    pub needs_network: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RussellPromptTemplate {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub variables: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MappedAsset {
    pub origin: String,
    pub origin_path: PathBuf,
    pub asset_type: MappedAssetType,
    pub hkask_manifest: Option<ProcessManifest>,
    pub hkask_template: Option<CompositionTemplate>,
    pub lexicon_terms: Vec<String>,
    pub provenance_hash: String,
    pub migration_timestamp: u64,
}

#[derive(Debug, Clone)]
pub enum MappedAssetType {
    SkillManifest,
    PromptTemplate,
    BotManifest,
}

#[derive(Debug, Clone)]
pub struct MigrationConfig {
    pub dry_run: bool,
    pub validate_only: bool,
    pub output_format: OutputFormat,
    pub transform_rules_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Default)]
pub enum OutputFormat {
    #[default]
    Yaml,
    Json,
    Mermaid,
}

pub struct RussellMapper {
    #[allow(dead_code)]
    provenance: ProvenanceManager,
}

impl Default for RussellMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl RussellMapper {
    pub fn new() -> Self {
        Self {
            provenance: ProvenanceManager::new(),
        }
    }

    pub fn analyze_skill_manifest(&self, yaml_path: &Path) -> Result<RussellSkillManifest> {
        let content = std::fs::read_to_string(yaml_path).map_err(|e| MapperError::IoError {
            path: yaml_path.to_path_buf(),
            source: e,
        })?;

        let manifest: RussellSkillManifest = serde_yaml::from_str(&content)?;

        if manifest.id.is_empty() {
            return Err(MapperError::MissingField {
                field: "id".to_string(),
            });
        }

        debug!(
            "Analyzed Russell skill manifest: {} v{}",
            manifest.id, manifest.version
        );

        Ok(manifest)
    }

    pub fn analyze_prompt_template(&self, j2_path: &Path) -> Result<RussellPromptTemplate> {
        let content = std::fs::read_to_string(j2_path).map_err(|e| MapperError::IoError {
            path: j2_path.to_path_buf(),
            source: e,
        })?;

        let (temperature, max_tokens, body) = Self::parse_jinja2_frontmatter(&content);

        let template = RussellPromptTemplate {
            name: j2_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string(),
            temperature,
            max_tokens,
            body,
            variables: Self::extract_jinja2_variables(&content),
        };

        debug!("Analyzed Russell prompt template: {}", template.name);

        Ok(template)
    }

    fn parse_jinja2_frontmatter(content: &str) -> (Option<f64>, Option<u32>, String) {
        let mut temperature: Option<f64> = None;
        let mut max_tokens: Option<u32> = None;
        let mut body_start = 0;
        let mut in_frontmatter = false;

        for (i, line) in content.lines().enumerate() {
            if line.trim().starts_with("[inference]") {
                in_frontmatter = true;
                continue;
            }

            if in_frontmatter {
                if line.trim().is_empty() && i > 0 {
                    body_start = content
                        .find(content.lines().nth(i).unwrap_or(""))
                        .unwrap_or(0);
                    break;
                } else if line.starts_with("temperature") {
                    if let Some(val) = line.split('=').nth(1) {
                        temperature = val.trim().parse().ok();
                    }
                } else if line.starts_with("max_tokens") {
                    if let Some(val) = line.split('=').nth(1) {
                        max_tokens = val.trim().parse().ok();
                    }
                } else if line.trim() == "---" {
                    body_start = content.find(line).unwrap_or(0) + line.len();
                    break;
                }
            }
        }

        let body = if body_start > 0 {
            content[body_start..].trim().to_string()
        } else {
            content.to_string()
        };

        (temperature, max_tokens, body)
    }

    fn extract_jinja2_variables(content: &str) -> Vec<String> {
        let mut variables = Vec::new();
        let re = regex::Regex::new(r"\{\{\s*([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();

        for cap in re.captures_iter(content) {
            if let Some(var) = cap.get(1) {
                let var_name = var.as_str().to_string();
                if !variables.contains(&var_name) {
                    variables.push(var_name);
                }
            }
        }
        variables
    }

    pub fn transform_to_hkask_manifest(
        &self,
        russell: &RussellSkillManifest,
    ) -> Result<ProcessManifest> {
        let mut steps = Vec::new();
        let mut ordinal = 1;

        for probe in &russell.probes {
            steps.push(crate::ports::ManifestStep {
                ordinal,
                action: crate::ports::Action::Execute,
                description: format!("Execute probe: {}", probe.id),
                template_ref: format!("probes/{}", probe.id),
                model_tier: None,
                mcp: Some("hkask-mcp-storage".to_string()),
                renderer: Some("shell".to_string()),
            });
            ordinal += 1;
        }

        for intervention in &russell.interventions {
            steps.push(crate::ports::ManifestStep {
                ordinal,
                action: crate::ports::Action::Execute,
                description: format!("Execute intervention: {}", intervention.id),
                template_ref: format!("interventions/{}", intervention.id),
                model_tier: None,
                mcp: Some("hkask-mcp-storage".to_string()),
                renderer: Some("shell".to_string()),
            });
            ordinal += 1;
        }

        if steps.is_empty() {
            steps.push(crate::ports::ManifestStep {
                ordinal: 1,
                action: crate::ports::Action::Populate,
                description: "Load knowledge into context".to_string(),
                template_ref: format!("skills/{}/knowledge", russell.id),
                model_tier: None,
                mcp: Some("hkask-mcp-memory".to_string()),
                renderer: Some("minijinja".to_string()),
            });
        }

        let manifest = ProcessManifest {
            id: format!("skill/{}", russell.id),
            name: russell.id.clone(),
            description: format!("Russell skill: {}", russell.id),
            steps,
        };

        debug!(
            "Transformed Russell skill '{}' to hKask manifest with {} steps",
            russell.id,
            manifest.steps.len()
        );

        Ok(manifest)
    }

    pub fn transform_to_hkask_template(
        &self,
        russell: &RussellPromptTemplate,
        _origin: &str,
    ) -> Result<CompositionTemplate> {
        let lexicon_terms = Self::infer_lexicon_terms(&russell.body);

        let contract = TemplateContract {
            input_fields: russell.variables.clone(),
            output_fields: vec!["rendered_document".to_string()],
        };

        let template = CompositionTemplate {
            id: format!("prompt/{}", russell.name),
            template_type: TemplateType::Prompt,
            lexicon_terms,
            source: russell.body.clone(),
            contract,
        };

        debug!(
            "Transformed Russell template '{}' to hKask template with {} lexicon terms",
            russell.name,
            template.lexicon_terms.len()
        );

        Ok(template)
    }

    fn infer_lexicon_terms(template_body: &str) -> Vec<String> {
        let mut terms = Vec::new();

        if template_body.contains("Subjective") || template_body.contains("Objective") {
            terms.push("observe".to_string());
            terms.push("assess".to_string());
        }

        if template_body.contains("Plan") || template_body.contains("ACTION") {
            terms.push("plan".to_string());
            terms.push("act".to_string());
        }

        if template_body.contains("Available skills") || template_body.contains("knowledge") {
            terms.push("discover".to_string());
            terms.push("recall".to_string());
        }

        if template_body.contains("events") || template_body.contains("Severity") {
            terms.push("monitor".to_string());
        }

        terms
    }

    pub fn compute_provenance_hash(&self, origin: &str, content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!("{}:{}", origin, content).as_bytes());
        hex::encode(hasher.finalize())
    }

    pub fn migrate_skill_manifest(
        &self,
        source_path: &Path,
        config: &MigrationConfig,
    ) -> Result<MappedAsset> {
        let russell = self.analyze_skill_manifest(source_path)?;

        let hkask_manifest = if config.validate_only {
            None
        } else {
            Some(self.transform_to_hkask_manifest(&russell)?)
        };

        let content = std::fs::read_to_string(source_path).map_err(|e| MapperError::IoError {
            path: source_path.to_path_buf(),
            source: e,
        })?;

        let provenance_hash = self.compute_provenance_hash("russell", &content);

        let asset = MappedAsset {
            origin: format!("russell/{}", russell.id),
            origin_path: source_path.to_path_buf(),
            asset_type: MappedAssetType::SkillManifest,
            hkask_manifest,
            hkask_template: None,
            lexicon_terms: vec![],
            provenance_hash,
            migration_timestamp: Utc::now().timestamp() as u64,
        };

        Ok(asset)
    }

    pub fn migrate_prompt_template(
        &self,
        source_path: &Path,
        config: &MigrationConfig,
    ) -> Result<MappedAsset> {
        let russell = self.analyze_prompt_template(source_path)?;

        let hkask_template = if config.validate_only {
            None
        } else {
            Some(self.transform_to_hkask_template(&russell, "russell")?)
        };

        let content = std::fs::read_to_string(source_path).map_err(|e| MapperError::IoError {
            path: source_path.to_path_buf(),
            source: e,
        })?;

        let provenance_hash = self.compute_provenance_hash("russell", &content);

        let lexicon_terms = hkask_template
            .as_ref()
            .map(|t| t.lexicon_terms.clone())
            .unwrap_or_default();

        let asset = MappedAsset {
            origin: format!("russell/{}", russell.name),
            origin_path: source_path.to_path_buf(),
            asset_type: MappedAssetType::PromptTemplate,
            hkask_manifest: None,
            hkask_template,
            lexicon_terms,
            provenance_hash,
            migration_timestamp: Utc::now().timestamp() as u64,
        };

        Ok(asset)
    }
}


