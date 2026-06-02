//! Port traits for registry and template execution
//
//! Defines the hexagonal architecture ports for template dispatch system.
//! Per architecture v0.21.0: Rust is the loom, YAML/Jinja2 is the thread.
//
//! `RegistryEntry`, `RegistryIndex`, and `RegistryError` are canonical in
//! `hkask_types::ports` and re-exported here for backward compatibility.

use hkask_types::CapabilityToken;
use serde_json::Value;

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
    #[error("Capability denied: {0}")]
    CapabilityDenied(String),
    #[error("Timeout: {0}")]
    Timeout(String),
}

pub type Result<T> = std::result::Result<T, TemplateError>;

/// Registry entry for template discovery
///
/// Canonical definition lives in `hkask_types::ports::RegistryEntry`.
/// Re-exported here for backward compatibility.
pub use hkask_types::ports::RegistryEntry;

/// Registry index port
///
/// Canonical definition lives in `hkask_types::ports::RegistryIndex`.
/// Re-exported here for backward compatibility.
pub use hkask_types::ports::RegistryIndex;

/// Registry error type
///
/// Canonical definition lives in `hkask_types::ports::RegistryError`.
/// Re-exported here for backward compatibility.
pub use hkask_types::ports::RegistryError;

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
}

/// MCP port for tool invocation
#[async_trait::async_trait]
pub trait McpPort: Send + Sync {
    async fn discover_tools(&self) -> Vec<String>;
    async fn invoke(&self, tool_name: &str, input: Value, token: &CapabilityToken)
    -> Result<Value>;
    async fn get_tool_info(&self, tool_name: &str) -> Option<ToolInfo>;
}
