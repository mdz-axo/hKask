//! ACP Port — Agent Communication Protocol hexagonal port
//!
//! Defines the interface for agent registration, A2A messaging,
//! and capability management.

use async_trait::async_trait;
use hkask_types::{CapabilityToken, WebID};

use crate::acp::{A2AMessage, AcpError};

/// ACP Port — Agent registration and A2A communication
///
/// # Hexagonal Architecture
///
/// This port is implemented by `AcpRuntime` (in-process) and can be
/// adapted for remote ACP servers via transport adapters.
#[async_trait]
pub trait AcpPort: Send + Sync {
    /// Register an agent with the ACP runtime
    ///
    /// # Arguments
    /// * `webid` — Agent's WebID
    /// * `agent_type` — "Bot" or "Replicant"
    /// * `capabilities` — Explicit capability list (no wildcards)
    ///
    /// # Returns
    /// * `Ok(CapabilityToken)` — Primary capability token
    /// * `Err(AcpError)` — Registration failure
    async fn register_agent(
        &self,
        webid: WebID,
        agent_type: &str,
        capabilities: Vec<String>,
    ) -> Result<CapabilityToken, AcpError>;

    /// Unregister an agent and revoke its capabilities
    async fn unregister_agent(&self, webid: &WebID) -> Result<(), AcpError>;

    /// Send an A2A message
    ///
    /// # Arguments
    /// * `msg` — A2A message (TemplateDispatch, TemplateResponse, MemoryArtifact)
    ///
    /// # Returns
    /// * `Ok(String)` — Correlation ID for tracking
    /// * `Err(AcpError)` — Send failure
    async fn send_message(&self, msg: A2AMessage) -> Result<String, AcpError>;

    /// List capabilities for a registered agent
    async fn list_capabilities(&self, webid: &WebID) -> Result<Vec<String>, AcpError>;

    /// Check if an agent is registered
    async fn is_registered(&self, webid: &WebID) -> bool;
}
