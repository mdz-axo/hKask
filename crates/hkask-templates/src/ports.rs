//! Port traits for registry and template execution
//!
//! Defines the hexagonal architecture ports for template dispatch system.
//! Per architecture v0.21.0: Rust is the loom, YAML/Jinja2 is the thread.

use hkask_types::TemplateType;
use hkask_types::{CapabilityToken, WebID};
use percent_encoding::percent_decode_str;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use url::Url;

/// Configuration for inference calls with timeout and retry
#[derive(Debug, Clone)]
pub struct InferenceConfig {
    /// Timeout for each inference call
    pub timeout: Duration,
    /// Maximum number of retries on transient failure
    pub max_retries: u32,
    /// Base delay for exponential backoff
    pub backoff_base: Duration,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            max_retries: 3,
            backoff_base: Duration::from_secs(1),
        }
    }
}

impl InferenceConfig {
    /// Calculate backoff delay for given attempt (exponential: 1s, 2s, 4s, ...)
    pub fn backoff_delay(&self, attempt: u32) -> Duration {
        self.backoff_base * 2u32.pow(attempt)
    }
}

/// Error type for template operations
#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    #[error("Template not found: {0}")]
    NotFound(String),
    #[error("Template exists but is not wired: {0}")]
    Unwired(String),
    #[error("Template entry is corrupt: {0}")]
    CorruptEntry(String),
    #[error("Render error: {0}")]
    Render(String),
    #[error("Manifest error: {0}")]
    Manifest(String),
    #[error("Inference error: {0}")]
    Inference(String),
    #[error("MCP error: {0}")]
    Mcp(String),
    #[error("Recursion limit exceeded (max depth: {max})")]
    RecursionLimit { max: u8 },
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Path traversal attempt: {0}")]
    PathTraversal(String),
    #[error("Sandbox violation: {0}")]
    SandboxViolation(String),
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),
    #[error("Capability denied: {0}")]
    CapabilityDenied(String),
    #[error("Timeout: {0}")]
    Timeout(String),
}

/// Manifest execution error with precise recovery semantics
#[derive(Debug, thiserror::Error)]
pub enum ManifestExecutionError {
    #[error("Step {ordinal} failed: {reason}")]
    StepFailure { ordinal: u32, reason: String },

    #[error("State transition invalid: {from} -> {to}")]
    InvalidTransition { from: String, to: String },

    #[error("CNS event emission failed at step {ordinal}")]
    CnsEmissionFailure { ordinal: u32 },

    #[error("Recursion depth {current} exceeded limit {max}")]
    DepthExceeded { current: u8, max: u8 },

    #[error("Capability check failed: {capability}")]
    CapabilityDenied { capability: String },

    #[error("Template selection failed: {reason}")]
    SelectionFailed { reason: String },

    #[error("Template population failed: {template_ref}: {reason}")]
    PopulationFailed {
        template_ref: String,
        reason: String,
    },

    #[error("Template execution failed: {mcp_target}: {reason}")]
    ExecutionFailed { mcp_target: String, reason: String },

    #[error("Manifest validation failed: {field}: {reason}")]
    ValidationFailed { field: String, reason: String },

    #[error("Manifest not found: {manifest_id}")]
    ManifestNotFound { manifest_id: String },

    #[error("YAML parse error: {reason}")]
    YamlParseError { reason: String },

    #[error("I/O error: {reason}")]
    IoError { reason: String },
}

impl ManifestExecutionError {
    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            ManifestExecutionError::StepFailure { .. } => false,
            ManifestExecutionError::InvalidTransition { .. } => false,
            ManifestExecutionError::CnsEmissionFailure { .. } => true,
            ManifestExecutionError::DepthExceeded { .. } => false,
            ManifestExecutionError::CapabilityDenied { .. } => false,
            ManifestExecutionError::SelectionFailed { .. } => true,
            ManifestExecutionError::PopulationFailed { .. } => true,
            ManifestExecutionError::ExecutionFailed { .. } => true,
            ManifestExecutionError::ValidationFailed { .. } => false,
            ManifestExecutionError::ManifestNotFound { .. } => false,
            ManifestExecutionError::YamlParseError { .. } => false,
            ManifestExecutionError::IoError { .. } => true,
        }
    }

    /// Get error category for CNS monitoring
    pub fn category(&self) -> &'static str {
        match self {
            ManifestExecutionError::StepFailure { .. } => "step_failure",
            ManifestExecutionError::InvalidTransition { .. } => "invalid_transition",
            ManifestExecutionError::CnsEmissionFailure { .. } => "cns_failure",
            ManifestExecutionError::DepthExceeded { .. } => "depth_exceeded",
            ManifestExecutionError::CapabilityDenied { .. } => "capability_denied",
            ManifestExecutionError::SelectionFailed { .. } => "selection_failed",
            ManifestExecutionError::PopulationFailed { .. } => "population_failed",
            ManifestExecutionError::ExecutionFailed { .. } => "execution_failed",
            ManifestExecutionError::ValidationFailed { .. } => "validation_failed",
            ManifestExecutionError::ManifestNotFound { .. } => "manifest_not_found",
            ManifestExecutionError::YamlParseError { .. } => "yaml_parse_error",
            ManifestExecutionError::IoError { .. } => "io_error",
        }
    }

    /// Convert to TemplateError for backward compatibility
    pub fn to_template_error(&self) -> TemplateError {
        match self {
            ManifestExecutionError::StepFailure { reason, .. } => {
                TemplateError::Manifest(reason.clone())
            }
            ManifestExecutionError::InvalidTransition { .. } => {
                TemplateError::Validation("Invalid state transition".to_string())
            }
            ManifestExecutionError::CnsEmissionFailure { .. } => {
                TemplateError::Manifest("CNS emission failed".to_string())
            }
            ManifestExecutionError::DepthExceeded { max, .. } => {
                TemplateError::RecursionLimit { max: *max }
            }
            ManifestExecutionError::CapabilityDenied { capability } => {
                TemplateError::CapabilityDenied(capability.clone())
            }
            ManifestExecutionError::SelectionFailed { reason } => {
                TemplateError::Manifest(reason.clone())
            }
            ManifestExecutionError::PopulationFailed { reason, .. } => {
                TemplateError::Render(reason.clone())
            }
            ManifestExecutionError::ExecutionFailed { reason, .. } => {
                TemplateError::Mcp(reason.clone())
            }
            ManifestExecutionError::ValidationFailed { reason, .. } => {
                TemplateError::Validation(reason.clone())
            }
            ManifestExecutionError::ManifestNotFound { manifest_id } => {
                TemplateError::NotFound(format!("Manifest {}", manifest_id))
            }
            ManifestExecutionError::YamlParseError { reason } => {
                TemplateError::Manifest(reason.clone())
            }
            ManifestExecutionError::IoError { reason } => TemplateError::Manifest(reason.clone()),
        }
    }
}

impl From<std::io::Error> for ManifestExecutionError {
    fn from(err: std::io::Error) -> Self {
        ManifestExecutionError::IoError {
            reason: err.to_string(),
        }
    }
}

impl From<serde_yaml::Error> for ManifestExecutionError {
    fn from(err: serde_yaml::Error) -> Self {
        ManifestExecutionError::YamlParseError {
            reason: err.to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, TemplateError>;

/// Dependency injection provider port trait
pub trait DependencyProvider: Send + Sync {
    /// Get a dependency by name
    fn get_dependency(&self, name: &str) -> Option<Value>;

    /// List all registered dependencies
    fn list_dependencies(&self) -> Vec<&str>;
}

/// In-memory dependency provider (default implementation)
pub struct InMemoryDependencyProvider {
    dependencies: HashMap<String, Value>,
}

impl InMemoryDependencyProvider {
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
        }
    }

    pub fn with_dependency(mut self, name: &str, value: Value) -> Self {
        self.dependencies.insert(name.to_string(), value);
        self
    }
}

impl Default for InMemoryDependencyProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl DependencyProvider for InMemoryDependencyProvider {
    fn get_dependency(&self, name: &str) -> Option<Value> {
        self.dependencies.get(name).cloned()
    }

    fn list_dependencies(&self) -> Vec<&str> {
        self.dependencies.keys().map(|s| s.as_str()).collect()
    }
}

/// Manifest step action types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Select,
    Populate,
    Execute,
}

impl serde::Serialize for Action {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for Action {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Action::from_str(&s).ok_or_else(|| {
            serde::de::Error::unknown_variant(
                &s,
                &[
                    "Select", "Populate", "Execute", "select", "populate", "execute",
                ],
            )
        })
    }
}

impl Action {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "select" => Some(Action::Select),
            "populate" => Some(Action::Populate),
            "execute" => Some(Action::Execute),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Action::Select => "select",
            Action::Populate => "populate",
            Action::Execute => "execute",
        }
    }
}

/// Manifest step definition
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ManifestStep {
    pub ordinal: u32,
    pub action: Action,
    pub description: String,
    pub template_ref: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_tier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub renderer: Option<String>,
}

/// Process manifest (YAML-based workflow definition)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProcessManifest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub steps: Vec<ManifestStep>,
}

impl ProcessManifest {
    /// Load manifest from YAML file
    pub fn load_from_yaml(path: &Path) -> Result<Self> {
        let yaml_content = std::fs::read_to_string(path).map_err(|e| {
            TemplateError::Manifest(format!(
                "Failed to read manifest file {}: {}",
                path.display(),
                e
            ))
        })?;

        serde_yaml::from_str(&yaml_content)
            .map_err(|e| TemplateError::Manifest(format!("Failed to parse manifest YAML: {}", e)))
    }
}

/// Manifest executor port
pub trait ManifestExecutor {
    fn load(&self, path: &Path) -> Result<ProcessManifest>;
    fn execute(&self, manifest: &ProcessManifest, input: Value) -> Result<Value>;
}

/// Manifest location abstraction (decouples file system from registry from network)
#[derive(Debug, Clone)]
pub enum ManifestLocation {
    /// Load from filesystem path
    FileSystem(PathBuf),
    /// Load from registry by template ID
    Registry(String),
    /// Load from URI (HTTP, IPFS, etc.)
    Uri(Url),
}

/// Manifest execution outcome (structured, not raw Value)
#[derive(Debug, Clone)]
pub struct ManifestOutcome {
    /// Unique execution ID for audit correlation
    pub execution_id: uuid::Uuid,
    /// Manifest ID that was executed
    pub manifest_id: String,
    /// Final state after execution
    pub final_state: Value,
    /// Number of steps completed
    pub steps_completed: u32,
    /// Execution duration
    pub duration: std::time::Duration,
    /// CNS events emitted during execution
    pub cns_events: Vec<(String, Value, f64)>,
}

impl ManifestOutcome {
    /// Create new manifest outcome
    pub fn new(
        execution_id: uuid::Uuid,
        manifest_id: String,
        final_state: Value,
        steps_completed: u32,
        duration: std::time::Duration,
    ) -> Self {
        Self {
            execution_id,
            manifest_id,
            final_state,
            steps_completed,
            duration,
            cns_events: vec![],
        }
    }

    /// Check if execution was successful (all steps completed)
    pub fn is_success(&self, total_steps: usize) -> bool {
        self.steps_completed as usize == total_steps
    }

    /// Get execution ID for correlation
    pub fn execution_id(&self) -> uuid::Uuid {
        self.execution_id
    }
}

/// Enhanced manifest executor port with structured outcomes
pub trait ManifestExecutorPort {
    /// Load manifest from abstract location
    fn load(&self, location: &ManifestLocation) -> Result<ProcessManifest>;

    /// Execute manifest with input state
    fn execute(&self, manifest: &ProcessManifest, input: Value) -> Result<ManifestOutcome>;

    /// Get execution trace for audit (by execution ID)
    fn get_trace(&self, execution_id: &uuid::Uuid) -> Option<ManifestOutcome>;
}

/// Manifest repository port (persistence abstraction)
pub trait ManifestRepository {
    /// Load manifest by ID
    fn load(&self, id: &str) -> Result<ProcessManifest>;

    /// Save manifest
    fn save(&self, manifest: &ProcessManifest) -> Result<()>;

    /// Delete manifest by ID
    fn delete(&self, id: &str) -> Result<()>;

    /// List all manifest IDs
    fn list(&self) -> Result<Vec<String>>;
}

/// Template composition definition
#[derive(Debug, Clone)]
pub struct CompositionTemplate {
    pub id: String,
    pub template_type: TemplateType,
    pub lexicon_terms: Vec<String>,
    pub source: String,
    pub contract: TemplateContract,
}

/// Template input/output contract
#[derive(Debug, Clone)]
pub struct TemplateContract {
    pub input_fields: Vec<String>,
    pub output_fields: Vec<String>,
}

/// Template renderer port
pub trait TemplateRenderer {
    fn load(&self, path: &Path) -> Result<CompositionTemplate>;
    fn render(&self, template: &CompositionTemplate, bindings: Value) -> Result<String>;
}

/// Registry entry for template discovery
#[derive(Debug, Clone)]
pub struct RegistryEntry {
    pub id: String,
    pub template_type: TemplateType,
    pub lexicon_terms: Vec<String>,
    pub description: String,
    pub source_path: String,
}

/// Registry index port
pub trait RegistryIndex {
    fn list(&self, domain_hint: Option<TemplateType>) -> Vec<RegistryEntry>;
    fn get(&self, id: &str) -> Result<RegistryEntry>;
    fn bootstrap_manifest(&self) -> Option<ProcessManifest>;
}

/// Inference port for LLM calls with timeout/retry support
pub trait InferencePort {
    fn call(&self, model_tier: &str, prompt: &str, config: &InferenceConfig) -> Result<Value>;

    /// Call with default configuration
    fn call_default(&self, model_tier: &str, prompt: &str) -> Result<Value> {
        self.call(model_tier, prompt, &InferenceConfig::default())
    }
}

/// MCP port for tool invocation
pub trait McpPort {
    fn discover_tools(&self) -> Vec<String>;
    fn invoke(&self, tool_name: &str, input: Value) -> Result<Value>;
}

/// CNS port for event emission
pub trait CnsPort {
    fn emit(&self, span: &str, outcome: Value, confidence: f64);
}

/// Security port for capability-based security checks
pub trait SecurityPort: Send + Sync {
    /// Verify capability token signature and delegation
    fn verify_signature(&self, token: &CapabilityToken, holder: &WebID) -> bool;

    /// Check capability for template operation
    fn check_template_capability(
        &self,
        token: &CapabilityToken,
        holder: &WebID,
        template_id: &str,
        current_time: i64,
    ) -> Result<()>;

    /// Check capability for manifest operation
    fn check_manifest_capability(
        &self,
        token: &CapabilityToken,
        holder: &WebID,
        manifest_id: &str,
        current_time: i64,
    ) -> Result<()>;

    /// Check capability for cascade operation
    fn check_cascade_capability(
        &self,
        token: &CapabilityToken,
        holder: &WebID,
        cascade_id: &str,
        current_time: i64,
    ) -> Result<()>;

    /// Check capability for stage operation
    fn check_stage_capability(
        &self,
        token: &CapabilityToken,
        holder: &WebID,
        stage_name: &str,
        current_time: i64,
    ) -> Result<()>;

    /// Check capability for stage operation with context nonce validation
    fn check_stage_capability_with_context(
        &self,
        token: &CapabilityToken,
        holder: &WebID,
        stage_name: &str,
        current_time: i64,
        expected_context: &str,
    ) -> Result<()> {
        // Default implementation: check stage capability and validate context nonce
        self.check_stage_capability(token, holder, stage_name, current_time)?;
        if !token.validate_context_nonce(expected_context) {
            return Err(TemplateError::CapabilityDenied(
                "Context nonce does not match execution context".to_string(),
            ));
        }
        Ok(())
    }

    /// Create attenuated capability for delegation
    fn attenuate_capability(
        &self,
        token: &CapabilityToken,
        new_to: WebID,
        current_time: i64,
    ) -> Option<CapabilityToken>;

    /// Validate template/manifest path (prevent path traversal)
    fn validate_path(&self, path: &str) -> Result<()>;

    /// Check recursion depth (prevent DoS via infinite recursion)
    fn check_recursion_depth(&self, current_depth: u8, max_depth: u8) -> Result<()>;

    /// Check energy budget (prevent resource exhaustion)
    fn check_energy_budget(&self, requested: u64, remaining: u64) -> Result<()>;
}

/// Capability attenuator port for runtime OCAP enforcement
/// 
/// Attenuates capabilities at template render time, following principle of least authority.
/// Per Miller: capabilities must be attenuated at use, not just declared at configuration.
pub trait CapabilityAttenuator: Send + Sync {
    /// Attenuate capability for specific template
    /// 
    /// Creates a more restrictive capability scoped to the template being rendered.
    /// The attenuated capability can only be used for that specific template.
    /// 
    /// # Arguments
    /// * `capability` - Original capability to attenuate
    /// * `template_id` - Template ID to scope capability to
    /// 
    /// # Returns
    /// * `Ok(attenuated_capability)` - New capability with template caveat added
    /// * `Err(TemplateError::CapabilityDenied)` - If attenuation not permitted
    fn attenuate_for_template(
        &self,
        capability: &hkask_types::CapabilityToken,
        template_id: &str,
    ) -> Result<hkask_types::CapabilityToken>;

    /// Verify attenuated capability has required template caveat
    fn verify_template_attenuation(
        &self,
        capability: &hkask_types::CapabilityToken,
        template_id: &str,
    ) -> bool;
}

/// Mock capability attenuator for testing
pub struct MockCapabilityAttenuator {
    pub should_attenuate: bool,
    pub should_verify: bool,
}

impl MockCapabilityAttenuator {
    pub fn new() -> Self {
        Self {
            should_attenuate: true,
            should_verify: true,
        }
    }
}

impl Default for MockCapabilityAttenuator {
    fn default() -> Self {
        Self::new()
    }
}

impl CapabilityAttenuator for MockCapabilityAttenuator {
    fn attenuate_for_template(
        &self,
        capability: &hkask_types::CapabilityToken,
        template_id: &str,
    ) -> Result<hkask_types::CapabilityToken> {
        if !self.should_attenuate {
            return Err(TemplateError::CapabilityDenied(
                "Attenuation disabled in mock".to_string(),
            ));
        }

        // Mock attenuation: create new token scoped to template
        // In production, this would use macaroon attenuation with template caveat
        Ok(hkask_types::CapabilityToken::new(
            hkask_types::CapabilityResource::Template,
            template_id.to_string(),
            hkask_types::CapabilityAction::Render,
            capability.delegated_from,
            capability.delegated_to,
            &capability.signature.as_bytes(), // Use existing signature as key
        ))
    }

    fn verify_template_attenuation(
        &self,
        capability: &hkask_types::CapabilityToken,
        template_id: &str,
    ) -> bool {
        if !self.should_verify {
            return false;
        }

        // Verify capability is scoped to template
        capability.resource == hkask_types::CapabilityResource::Template
            && capability.resource_id == template_id
    }
}

/// Energy calibrator port for manifest energy budget analysis
/// 
/// Analyzes manifest energy budgets and provides calibration recommendations.
/// Per Cockburn: CLI is an adapter that calls this port, not direct implementation.
pub trait EnergyCalibrator: Send + Sync {
    /// Calibrate energy for a single manifest
    /// 
    /// # Arguments
    /// * `manifest_path` - Path to manifest YAML file
    /// 
    /// # Returns
    /// * `Ok(EnergyCalibrationReport)` - Calibration results with recommendations
    /// * `Err(TemplateError)` - If manifest cannot be loaded or analyzed
    fn calibrate_manifest(&self, manifest_path: &std::path::Path) -> Result<EnergyCalibrationReport>;

    /// Calibrate energy for all manifests in a directory
    /// 
    /// # Arguments
    /// * `manifest_dir` - Directory containing manifest YAML files
    /// 
    /// # Returns
    /// * `Ok(Vec<EnergyCalibrationReport>)` - Calibration results for all manifests
    /// * `Err(TemplateError)` - If directory cannot be read
    fn calibrate_all_manifests(&self, manifest_dir: &std::path::Path) -> Result<Vec<EnergyCalibrationReport>>;
}

/// Energy calibration report
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EnergyCalibrationReport {
    /// Manifest identifier
    pub manifest_id: String,
    /// Current energy cap from manifest
    pub current_cap: u64,
    /// Recommended energy cap based on analysis
    pub recommended_cap: u64,
    /// Cap utilization percentage (0-100)
    pub cap_utilization: f64,
    /// Estimated energy cost for manifest execution
    pub estimated_cost: u64,
    /// Number of steps in manifest
    pub steps_count: usize,
    /// Cost per token configuration
    pub cost_per_token: f64,
    /// Alert threshold configuration (0-1)
    pub alert_threshold: f64,
    /// Calibration recommendation
    pub recommendation: String,
}

/// Default energy calibrator implementation
pub struct DefaultEnergyCalibrator;

impl DefaultEnergyCalibrator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefaultEnergyCalibrator {
    fn default() -> Self {
        Self::new()
    }
}

impl EnergyCalibrator for DefaultEnergyCalibrator {
    fn calibrate_manifest(&self, manifest_path: &std::path::Path) -> Result<EnergyCalibrationReport> {
        use serde_yaml::Value as YamlValue;
        
        let content = std::fs::read_to_string(manifest_path)
            .map_err(|e| TemplateError::Manifest(format!("Failed to read manifest: {}", e)))?;
        
        let yaml: YamlValue = serde_yaml::from_str(&content)
            .map_err(|e| TemplateError::Manifest(format!("Failed to parse YAML: {}", e)))?;
        
        // Extract energy configuration
        let energy_config = yaml.get("energy")
            .ok_or_else(|| TemplateError::Manifest("No energy configuration found".to_string()))?;
        
        let cap = energy_config.get("cap")
            .and_then(|v| v.as_u64())
            .unwrap_or(10000);
        
        let cost_per_token = energy_config.get("cost_per_token")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.25);
        
        let alert_threshold = energy_config.get("alert_threshold")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.8);
        
        // Analyze steps for energy requirements
        let steps = yaml.get("steps")
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
            manifest_id: manifest_path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown").to_string(),
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

    fn calibrate_all_manifests(&self, manifest_dir: &std::path::Path) -> Result<Vec<EnergyCalibrationReport>> {
        let mut reports = Vec::new();
        
        if !manifest_dir.exists() {
            return Err(TemplateError::Manifest(format!(
                "Manifest directory does not exist: {:?}",
                manifest_dir
            )));
        }
        
        let entries = std::fs::read_dir(manifest_dir)
            .map_err(|e| TemplateError::Manifest(format!("Failed to read directory: {}", e)))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| {
                TemplateError::Manifest(format!("Failed to read directory entry: {}", e))
            })?;
            
            let path = entry.path();
            
            // Only process .yaml files
            if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                let report = self.calibrate_manifest(&path)?;
                reports.push(report);
            }
        }
        
        Ok(reports)
    }
}

/// Maximum Matroshka nesting depth (configurable per template)
pub const DEFAULT_MATROSHKA_LIMIT: u8 = 7;

/// Default model tier for fast local inference
pub const FAST_LOCAL_MODEL: &str = "fast_local";

/// Mock security port for testing
#[derive(Default)]
pub struct MockSecurityPort {
    pub should_verify: bool,
    pub should_check_template: bool,
    pub should_check_manifest: bool,
    pub should_check_cascade: bool,
    pub should_check_stage: bool,
    pub should_attenuate: bool,
    pub should_validate_path: bool,
    pub should_check_depth: bool,
    pub should_check_energy: bool,
}

impl MockSecurityPort {
    pub fn new() -> Self {
        Self {
            should_verify: true,
            should_check_template: true,
            should_check_manifest: true,
            should_check_cascade: true,
            should_check_stage: true,
            should_attenuate: true,
            should_validate_path: true,
            should_check_depth: true,
            should_check_energy: true,
        }
    }

    pub fn with_verification(mut self, should_verify: bool) -> Self {
        self.should_verify = should_verify;
        self
    }

    pub fn with_template_check(mut self, should_check: bool) -> Self {
        self.should_check_template = should_check;
        self
    }

    pub fn with_path_validation(mut self, should_validate: bool) -> Self {
        self.should_validate_path = should_validate;
        self
    }
}

impl SecurityPort for MockSecurityPort {
    fn verify_signature(&self, _token: &CapabilityToken, _holder: &WebID) -> bool {
        self.should_verify
    }

    fn check_template_capability(
        &self,
        _token: &CapabilityToken,
        _holder: &WebID,
        _template_id: &str,
        _current_time: i64,
    ) -> Result<()> {
        if self.should_check_template {
            Ok(())
        } else {
            Err(TemplateError::CapabilityDenied(
                "Mock security check failed".to_string(),
            ))
        }
    }

    fn check_manifest_capability(
        &self,
        _token: &CapabilityToken,
        _holder: &WebID,
        _manifest_id: &str,
        _current_time: i64,
    ) -> Result<()> {
        if self.should_check_manifest {
            Ok(())
        } else {
            Err(TemplateError::CapabilityDenied(
                "Mock security check failed".to_string(),
            ))
        }
    }

    fn check_cascade_capability(
        &self,
        _token: &CapabilityToken,
        _holder: &WebID,
        _cascade_id: &str,
        _current_time: i64,
    ) -> Result<()> {
        if self.should_check_cascade {
            Ok(())
        } else {
            Err(TemplateError::CapabilityDenied(
                "Mock security check failed".to_string(),
            ))
        }
    }

    fn check_stage_capability(
        &self,
        _token: &CapabilityToken,
        _holder: &WebID,
        _stage_name: &str,
        _current_time: i64,
    ) -> Result<()> {
        if self.should_check_stage {
            Ok(())
        } else {
            Err(TemplateError::CapabilityDenied(
                "Mock security check failed".to_string(),
            ))
        }
    }

    fn attenuate_capability(
        &self,
        token: &CapabilityToken,
        new_to: WebID,
        _current_time: i64,
    ) -> Option<CapabilityToken> {
        if self.should_attenuate {
            // Create a simple attenuated token for testing
            Some(CapabilityToken {
                id: format!("{}-attenuated", token.id),
                resource: token.resource,
                resource_id: token.resource_id.clone(),
                action: token.action,
                delegated_from: token.delegated_to.clone(),
                delegated_to: new_to,
                signature: token.signature.clone(),
                expires_at: token.expires_at,
                attenuation_level: token.attenuation_level + 1,
                max_attenuation: token.max_attenuation,
                context_nonce: format!("{}-attenuated", token.context_nonce),
            })
        } else {
            None
        }
    }

    fn validate_path(&self, path: &str) -> Result<()> {
        if self.should_validate_path {
            // Match SecurityAdapter validation: URL decode, double-decode, normalize

            // URL decode the path first
            let decoded = percent_decode_str(path)
                .decode_utf8()
                .map_err(|_| TemplateError::PathTraversal("Invalid UTF-8 in path".to_string()))?;

            // Double-decode to catch %252e%252e attacks
            let fully_decoded = percent_decode_str(decoded.as_ref())
                .decode_utf8()
                .unwrap_or_else(|_| decoded.clone());

            let normalized_path = fully_decoded.as_ref();

            // Reject absolute paths
            if normalized_path.starts_with('/') || normalized_path.starts_with('\\') {
                return Err(TemplateError::PathTraversal(format!(
                    "Absolute path not allowed: {}",
                    path
                )));
            }

            // Reject path traversal patterns
            const PATH_TRAVERSAL_PATTERNS: &[&str] =
                &["..", "/etc/", "/proc/", "/sys/", "//", "\\..", "/.."];
            for pattern in PATH_TRAVERSAL_PATTERNS {
                if normalized_path.contains(pattern) {
                    return Err(TemplateError::PathTraversal(format!(
                        "Path traversal pattern detected: {}",
                        pattern
                    )));
                }
            }

            // Reject null bytes
            if normalized_path.contains('\0') {
                return Err(TemplateError::PathTraversal(
                    "Null byte not allowed".to_string(),
                ));
            }

            Ok(())
        } else {
            Err(TemplateError::PathTraversal(
                "Mock path validation failed".to_string(),
            ))
        }
    }

    fn check_recursion_depth(&self, current_depth: u8, max_depth: u8) -> Result<()> {
        if self.should_check_depth && current_depth > max_depth {
            Err(TemplateError::RecursionLimit { max: max_depth })
        } else {
            Ok(())
        }
    }

    fn check_energy_budget(&self, requested: u64, remaining: u64) -> Result<()> {
        if self.should_check_energy && requested > remaining {
            Err(TemplateError::Manifest(format!(
                "Energy budget exceeded: requested {}, remaining {}",
                requested, remaining
            )))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inference_config_default() {
        let config = InferenceConfig::default();

        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.backoff_base, Duration::from_secs(1));
    }

    #[test]
    fn test_inference_config_backoff() {
        let config = InferenceConfig::default();

        // Exponential backoff: 1s, 2s, 4s, 8s, ...
        assert_eq!(config.backoff_delay(0), Duration::from_secs(1));
        assert_eq!(config.backoff_delay(1), Duration::from_secs(2));
        assert_eq!(config.backoff_delay(2), Duration::from_secs(4));
        assert_eq!(config.backoff_delay(3), Duration::from_secs(8));
    }

    #[test]
    fn test_inference_config_custom() {
        let config = InferenceConfig {
            timeout: Duration::from_secs(60),
            max_retries: 5,
            backoff_base: Duration::from_millis(500),
        };

        assert_eq!(config.backoff_delay(0), Duration::from_millis(500));
        assert_eq!(config.backoff_delay(1), Duration::from_secs(1));
        assert_eq!(config.backoff_delay(2), Duration::from_secs(2));
    }

    #[test]
    fn test_dependency_provider_new() {
        let provider = InMemoryDependencyProvider::new();
        assert!(provider.list_dependencies().is_empty());
    }

    #[test]
    fn test_dependency_provider_with_dependency() {
        let provider = InMemoryDependencyProvider::new()
            .with_dependency("key1", serde_json::json!("value1"))
            .with_dependency("key2", serde_json::json!("value2"));

        assert_eq!(
            provider.get_dependency("key1"),
            Some(serde_json::json!("value1"))
        );
        assert_eq!(
            provider.get_dependency("key2"),
            Some(serde_json::json!("value2"))
        );
        assert_eq!(provider.get_dependency("key3"), None);
    }

    #[test]
    fn test_dependency_provider_list() {
        let provider = InMemoryDependencyProvider::new()
            .with_dependency("a", serde_json::json!(1))
            .with_dependency("b", serde_json::json!(2));

        let deps = provider.list_dependencies();
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&"a"));
        assert!(deps.contains(&"b"));
    }

    #[test]
    fn test_process_manifest_load_from_yaml() {
        use std::path::PathBuf;

        let yaml_path = PathBuf::from("registry/manifests/dispatch.yaml");
        if yaml_path.exists() {
            let manifest = ProcessManifest::load_from_yaml(&yaml_path).unwrap();

            assert_eq!(manifest.id, "registry/dispatch");
            assert_eq!(manifest.name, "Registry Dispatch");
            assert_eq!(manifest.steps.len(), 3);

            assert_eq!(manifest.steps[0].action, Action::Select);
            assert_eq!(manifest.steps[0].template_ref, "prompt/selector");

            assert_eq!(manifest.steps[1].action, Action::Populate);

            assert_eq!(manifest.steps[2].action, Action::Execute);
        }
    }

    #[test]
    fn test_manifest_execution_error_retryable() {
        let err = ManifestExecutionError::StepFailure {
            ordinal: 1,
            reason: "test".to_string(),
        };
        assert!(!err.is_retryable());
        assert_eq!(err.category(), "step_failure");
    }

    #[test]
    fn test_manifest_execution_error_retryable_cases() {
        // Retryable errors
        assert!(ManifestExecutionError::CnsEmissionFailure { ordinal: 1 }.is_retryable());
        assert!(
            ManifestExecutionError::SelectionFailed {
                reason: "test".to_string()
            }
            .is_retryable()
        );
        assert!(
            ManifestExecutionError::PopulationFailed {
                template_ref: "test".to_string(),
                reason: "test".to_string()
            }
            .is_retryable()
        );
        assert!(
            ManifestExecutionError::ExecutionFailed {
                mcp_target: "test".to_string(),
                reason: "test".to_string()
            }
            .is_retryable()
        );
        assert!(
            ManifestExecutionError::IoError {
                reason: "test".to_string()
            }
            .is_retryable()
        );

        // Non-retryable errors
        assert!(
            !ManifestExecutionError::DepthExceeded {
                current: 10,
                max: 7
            }
            .is_retryable()
        );
        assert!(
            !ManifestExecutionError::CapabilityDenied {
                capability: "test".to_string()
            }
            .is_retryable()
        );
        assert!(
            !ManifestExecutionError::ValidationFailed {
                field: "test".to_string(),
                reason: "test".to_string()
            }
            .is_retryable()
        );
    }

    #[test]
    fn test_manifest_execution_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let manifest_err: ManifestExecutionError = io_err.into();
        assert!(matches!(
            manifest_err,
            ManifestExecutionError::IoError { .. }
        ));
        assert!(manifest_err.is_retryable());
    }

    #[test]
    fn test_manifest_execution_error_to_template_error() {
        let err = ManifestExecutionError::DepthExceeded {
            current: 10,
            max: 7,
        };
        let template_err = err.to_template_error();
        assert!(matches!(
            template_err,
            TemplateError::RecursionLimit { max: 7 }
        ));
    }
}
