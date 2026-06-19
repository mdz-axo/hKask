//! Port traits for registry and template execution
//
//! Defines the hexagonal architecture ports for template dispatch system.
//! Per architecture v0.21.0: Rust is the loom, YAML/Jinja2 is the thread.
//
//! `RegistryEntry`, `RegistryIndex`, and `RegistryError` are canonical in
//! `hkask_types::ports` and re-exported here for backward compatibility.

use hkask_types::DelegationToken;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

/// Error type for template operations
#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    #[error("Template not found: {0}")]
    NotFound(String),

    #[error("Render error: {0}")]
    Render(String),
    #[error("Manifest error: {0}")]
    Manifest(String),
    #[error("Database error: {0}")]
    Database(#[from] hkask_types::InfrastructureError),
    #[error("Inference error: {0}")]
    Inference(#[from] hkask_types::InferenceError),
    #[error("MCP error: {0}")]
    Mcp(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Path traversal attempt: {0}")]
    PathTraversal(String),
    #[error("Sandbox violation: {0}")]
    SandboxViolation(String),
    #[error("Capability denied: {0}")]
    CapabilityDenied(String),
}

pub type Result<T> = std::result::Result<T, TemplateError>;

use hkask_types::ports::ToolInfo;

/// MCP port for tool invocation
///
/// Object-safe trait for dynamic dispatch. Uses ``Pin<Box<dyn Future>>``
/// return types to be dyn-compatible, matching the pattern used by
/// `InferencePort` in `hkask_types::ports`.
///
/// P1 fix: previously used `impl Future` return types which prevented
/// dyn-dispatch. Now uses boxed futures, enabling `Arc<dyn McpPort>`
/// and removing the generic parameter from `ManifestExecutor`.
pub trait McpPort: Send + Sync {
    /// Discover available tools on the connected MCP server.
    fn discover_tools(&self) -> Pin<Box<dyn Future<Output = Vec<String>> + Send>>;

    /// Invoke an MCP tool by name with the given input and delegation token.
    fn invoke(
        &self,
        tool_name: &str,
        input: Value,
        token: &DelegationToken,
    ) -> Pin<Box<dyn Future<Output = Result<Value>> + Send>>;

    /// Get metadata for a specific tool.
    fn get_tool_info(
        &self,
        tool_name: &str,
    ) -> Pin<Box<dyn Future<Output = Option<ToolInfo>> + Send>>;
}
