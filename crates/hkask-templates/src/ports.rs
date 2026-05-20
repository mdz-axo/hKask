//! Port traits for registry and template execution
//!
//! Defines the hexagonal architecture ports for template dispatch system.
//! Per architecture v0.21.0: Rust is the loom, YAML/Jinja2 is the thread.

use hkask_types::TemplateType;
use hkask_types::{CapabilityToken, WebID};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Action {
    Select,
    Populate,
    Execute,
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
            TemplateError::Manifest(format!("Failed to read manifest file {}: {}", path.display(), e))
        })?;
        
        serde_yaml::from_str(&yaml_content).map_err(|e| {
            TemplateError::Manifest(format!("Failed to parse manifest YAML: {}", e))
        })
    }
}

/// Manifest executor port
pub trait ManifestExecutor {
    fn load(&self, path: &Path) -> Result<ProcessManifest>;
    fn execute(&self, manifest: &ProcessManifest, input: Value) -> Result<Value>;
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
            use hkask_types::CapabilityAction;
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
            // Simple validation: reject absolute paths and traversal
            if path.starts_with('/') || path.starts_with('\\') || path.contains("..") {
                Err(TemplateError::PathTraversal(format!(
                    "Invalid path: {}",
                    path
                )))
            } else {
                Ok(())
            }
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

        assert_eq!(provider.get_dependency("key1"), Some(serde_json::json!("value1")));
        assert_eq!(provider.get_dependency("key2"), Some(serde_json::json!("value2")));
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
}
