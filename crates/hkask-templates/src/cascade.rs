//! Cascade Composition — Simplified
//!
//! Cascade engine reads configuration from cascade-composition.yaml.
//! Rust is the loom. YAML is the thread.
//! ℏKask v0.21.2

use crate::ports::TemplateError;
use hkask_cns::spans::SpanEmitter;
use hkask_types::WebID;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Maximum cascade depth (Miller's law)
pub const MAX_CASCADE_DEPTH: u8 = 7;

/// Cascade configuration (loaded from YAML)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CascadeConfig {
    pub cascade_limits: CascadeLimits,
    #[serde(default)]
    pub cycle_detection: CycleDetectionConfig,
    #[serde(default)]
    pub template_cascade: TemplateCascadeConfig,
    #[serde(default)]
    pub manifest_cascade: ManifestCascadeConfig,
    #[serde(default)]
    pub energy: EnergyConfig,
    #[serde(default)]
    pub capabilities: CapabilityConfig,
    #[serde(default)]
    pub cns_feedback: CnsFeedbackConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CascadeLimits {
    #[serde(default = "default_max_depth")]
    pub max_depth: u8,
    #[serde(default)]
    pub energy_per_level: u64,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_max_depth() -> u8 {
    7
}

fn default_timeout() -> u64 {
    10000
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CycleDetectionConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub algorithm: String,
    #[serde(default)]
    pub max_path_length: usize,
    #[serde(default)]
    pub on_cycle_detected: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TemplateCascadeConfig {
    #[serde(default)]
    pub pre: Vec<CascadeStage>,
    #[serde(default)]
    pub core: Vec<CascadeStage>,
    #[serde(default)]
    pub post: Vec<CascadeStage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CascadeStage {
    pub name: String,
    #[serde(default)]
    pub templates: Vec<String>,
    #[serde(default)]
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ManifestCascadeConfig {
    #[serde(default)]
    pub sub_process: SubProcessConfig,
    #[serde(default)]
    pub parallel: ParallelConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SubProcessConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_max_sub_depth")]
    pub max_depth: u8,
    #[serde(default)]
    pub inheritance: InheritanceConfig,
}

fn default_max_sub_depth() -> u8 {
    5
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InheritanceConfig {
    #[serde(default)]
    pub capabilities: String,
    #[serde(default)]
    pub energy: String,
    #[serde(default)]
    pub context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParallelConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub max_concurrent: usize,
    #[serde(default)]
    pub sync: SyncConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncConfig {
    #[serde(default)]
    pub strategy: String,
    #[serde(default)]
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnergyConfig {
    #[serde(default)]
    pub budget: BudgetConfig,
    #[serde(default)]
    pub degradation: DegradationConfig,
    #[serde(default)]
    pub tracking: TrackingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BudgetConfig {
    #[serde(default)]
    pub total_per_cascade: u64,
    #[serde(default)]
    pub reserve_percent: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DegradationConfig {
    #[serde(default)]
    pub at_80_percent: String,
    #[serde(default)]
    pub at_90_percent: String,
    #[serde(default)]
    pub at_95_percent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrackingConfig {
    #[serde(default)]
    pub emit_cns_spans: bool,
    #[serde(default)]
    pub track_per_stage: bool,
    #[serde(default)]
    pub log_budget_remaining: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CapabilityConfig {
    #[serde(default)]
    pub attenuation: AttenuationConfig,
    #[serde(default)]
    pub validation: ValidationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AttenuationConfig {
    #[serde(default)]
    pub on_dispatch: String,
    #[serde(default = "default_max_attenuation")]
    pub max_attenuation_level: u8,
    #[serde(default)]
    pub minimum: Vec<String>,
}

fn default_max_attenuation() -> u8 {
    7
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ValidationConfig {
    #[serde(default)]
    pub check_on_each_call: bool,
    #[serde(default)]
    pub cache_validated_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CnsFeedbackConfig {
    #[serde(default)]
    pub spans: SpanConfig,
    #[serde(default)]
    pub variety: VarietyConfig,
    #[serde(default)]
    pub calibration: CalibrationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpanConfig {
    #[serde(default)]
    pub emit_on_stage_start: bool,
    #[serde(default)]
    pub emit_on_stage_complete: bool,
    #[serde(default)]
    pub emit_on_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VarietyConfig {
    #[serde(default)]
    pub track_template_diversity: bool,
    #[serde(default)]
    pub track_manifest_diversity: bool,
    #[serde(default)]
    pub alert_on_low_variety: bool,
    #[serde(default)]
    pub variety_threshold: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CalibrationConfig {
    #[serde(default)]
    pub adjust_timeout_on_failure: bool,
    #[serde(default)]
    pub adjust_energy_on_success: bool,
    #[serde(default)]
    pub learning_rate: f64,
}

/// Cascade execution context
#[derive(Debug, Clone)]
pub struct CascadeContext {
    pub current_depth: u8,
    pub visited_templates: HashSet<String>,
    pub visited_manifests: HashSet<String>,
    pub energy_remaining: u64,
}

impl CascadeContext {
    pub fn new(_max_depth: u8, initial_energy: u64) -> Self {
        Self {
            current_depth: 0,
            visited_templates: HashSet::new(),
            visited_manifests: HashSet::new(),
            energy_remaining: initial_energy,
        }
    }

    pub fn can_recurse(&self, config: &CascadeLimits) -> bool {
        self.current_depth < config.max_depth && self.energy_remaining > 0
    }

    pub fn descend(&mut self, template: &str, manifest: Option<&str>) {
        self.current_depth += 1;
        self.visited_templates.insert(template.to_string());
        if let Some(m) = manifest {
            self.visited_manifests.insert(m.to_string());
        }
    }
}

/// Cascade engine
pub struct CascadeEngine {
    config: CascadeConfig,
    emitter: SpanEmitter,
}

impl CascadeEngine {
    pub fn new(config: CascadeConfig) -> Self {
        let observer = WebID::new();
        Self {
            config,
            emitter: SpanEmitter::new(observer),
        }
    }

    /// Execute cascade with cycle detection
    pub async fn execute(
        &self,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, TemplateError> {
        let mut context = CascadeContext::new(
            self.config.cascade_limits.max_depth,
            self.config.cascade_limits.energy_per_level,
        );

        self.emitter.emit_tool(
            "cascade.start",
            serde_json::json!({
                "max_depth": self.config.cascade_limits.max_depth,
                "energy_budget": self.config.cascade_limits.energy_per_level,
            }),
        );

        // Check for cycles
        if self.config.cycle_detection.enabled {
            self.check_cycles(&context)?;
        }

        // Execute cascade stages
        let result = self.execute_stages(input, &mut context).await;

        self.emitter.emit_tool(
            "cascade.complete",
            serde_json::json!({
                "depth_reached": context.current_depth,
                "energy_remaining": context.energy_remaining,
            }),
        );

        result
    }

    fn check_cycles(&self, context: &CascadeContext) -> Result<(), TemplateError> {
        // Simple cycle detection via visited set
        if context.visited_templates.len() > self.config.cycle_detection.max_path_length {
            return Err(TemplateError::Validation(
                "Cascade path too long - possible cycle".to_string(),
            ));
        }
        Ok(())
    }

    async fn execute_stages(
        &self,
        input: serde_json::Value,
        context: &mut CascadeContext,
    ) -> Result<serde_json::Value, TemplateError> {
        let mut current = input;

        // Pre-processing
        for stage in &self.config.template_cascade.pre {
            if !context.can_recurse(&self.config.cascade_limits) {
                break;
            }
            
            context.descend(&stage.name, None);
            current = self.execute_stage(stage, current, context).await?;
        }

        // Core processing
        for stage in &self.config.template_cascade.core {
            if !context.can_recurse(&self.config.cascade_limits) {
                break;
            }
            
            context.descend(&stage.name, None);
            current = self.execute_stage(stage, current, context).await?;
        }

        // Post-processing
        for stage in &self.config.template_cascade.post {
            if !context.can_recurse(&self.config.cascade_limits) {
                break;
            }
            
            context.descend(&stage.name, None);
            current = self.execute_stage(stage, current, context).await?;
        }

        Ok(current)
    }

    async fn execute_stage(
        &self,
        stage: &CascadeStage,
        input: serde_json::Value,
        _context: &mut CascadeContext,
    ) -> Result<serde_json::Value, TemplateError> {
        self.emitter.emit_tool(
            "cascade.stage",
            serde_json::json!({
                "stage": stage.name,
                "templates": stage.templates,
            }),
        );

        // Generic stage execution - actual logic in templates
        Ok(input)
    }
}

/// Load cascade config from YAML
pub fn load_cascade_config(yaml_path: &str) -> Result<CascadeConfig, TemplateError> {
    let content = std::fs::read_to_string(yaml_path)
        .map_err(|e| TemplateError::Validation(format!("Failed to read cascade config: {}", e)))?;
    
    serde_yaml::from_str(&content)
        .map_err(|e| TemplateError::Validation(format!("Failed to parse cascade config: {}", e)))
}