//! Port traits for registry and template execution
//!
//! Defines the hexagonal architecture ports for template dispatch system.
//! Per architecture v0.21.0: Rust is the loom, YAML/Jinja2 is the thread.

use hkask_types::TemplateType;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

/// Configuration for inference calls with timeout and retry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    pub timeout: Duration,
    pub max_retries: u32,
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
    pub fn backoff_delay(&self, attempt: u32) -> Duration {
        self.backoff_base * 2u32.pow(attempt)
    }
}

/// Error type for template operations
#[derive(Debug, Clone, thiserror::Error)]
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
    #[error("Database error: {0}")]
    Database(String),
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

/// Manifest step action types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Select,
    Populate,
    Execute,
}

impl std::str::FromStr for Action {
    type Err = TemplateError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "select" => Ok(Action::Select),
            "populate" => Ok(Action::Populate),
            "execute" => Ok(Action::Execute),
            other => Err(TemplateError::Validation(format!(
                "Unknown action: {}",
                other
            ))),
        }
    }
}

impl Action {
    pub fn as_str(&self) -> &'static str {
        match self {
            Action::Select => "select",
            Action::Populate => "populate",
            Action::Execute => "execute",
        }
    }
}

/// Manifest step definition
#[derive(Debug, Clone)]
pub struct ManifestStep {
    pub ordinal: u32,
    pub action: Action,
    pub description: String,
    pub template_ref: String,
    pub model_tier: Option<String>,
    pub mcp: Option<String>,
    pub renderer: Option<String>,
}

/// Process manifest (YAML-based workflow definition)
#[derive(Debug, Clone)]
pub struct ProcessManifest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub steps: Vec<ManifestStep>,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateContract {
    pub input_fields: Vec<String>,
    pub output_fields: Vec<String>,
}

/// Registry entry for template discovery
#[derive(Debug, Clone)]
pub struct RegistryEntry {
    pub id: String,
    pub template_type: TemplateType,
    pub lexicon_terms: Vec<String>,
    pub description: String,
    pub source_path: String,
    /// Required capabilities for this template (R4: Capability Intersection)
    pub required_capabilities: Vec<String>,
}

/// Registry index port
pub trait RegistryIndex {
    fn list(&self, domain_hint: Option<TemplateType>) -> Vec<RegistryEntry>;
    fn get(&self, id: &str) -> Result<RegistryEntry>;
    fn bootstrap_manifest(&self) -> Option<ProcessManifest>;
}

/// Tool information metadata
#[derive(Debug, Clone)]
pub struct ToolInfo {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Input schema (JSON Schema)
    pub input_schema: Value,
    /// Server that provides this tool
    pub server_id: String,
    /// Required capability (if any)
    pub required_capability: Option<String>,
    /// Rate limit hint (tools/min)
    pub rate_limit_hint: Option<u32>,
}

/// MCP port for tool invocation
#[async_trait::async_trait]
pub trait McpPort: Send + Sync {
    async fn discover_tools(&self) -> Vec<String>;
    async fn invoke(&self, tool_name: &str, input: Value) -> Result<Value>;
    async fn get_tool_info(&self, tool_name: &str) -> Option<ToolInfo>;
}

/// CNS port for event emission — re-export of CnsEmit from hkask-cns
pub use hkask_cns::CnsEmit as CnsPort;

/// Memory context fragment for deduplication
#[derive(Debug, Clone)]
pub struct MemoryFragment {
    pub content: String,
    pub source: String,
    pub confidence: f64,
}

/// Maximum Matroshka nesting depth (configurable per template)
pub const DEFAULT_MATROSHKA_LIMIT: u8 = 7;

/// Default model tier for fast local inference
pub const FAST_LOCAL_MODEL: &str = "fast_local";
