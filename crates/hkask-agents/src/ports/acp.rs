//! ACP Port — Agent Communication Protocol hexagonal port
//!
//! Defines the interface for agent registration, A2A messaging,
//! and capability management.

use async_trait::async_trait;
use hkask_types::{AgentKind, DelegationToken, WebID};

use crate::acp::{A2AMessage, AcpError};

/// ACP Port — Agent registration and A2A communication
///
/// # Hexagonal Architecture
///
/// This port is implemented by `AcpRuntime` (in-process) and can be
/// adapted for remote ACP servers via transport adapters.
#[async_trait]
pub trait AcpPort: Send + Sync {
    async fn register_agent(
        &self,
        webid: WebID,
        agent_type: AgentKind,
        capabilities: Vec<String>,
    ) -> Result<DelegationToken, AcpError>;

    async fn unregister_agent(&self, webid: &WebID) -> Result<(), AcpError>;

    async fn send_message(&self, msg: A2AMessage) -> Result<String, AcpError>;

    async fn list_capabilities(&self, webid: &WebID) -> Result<Vec<String>, AcpError>;

    async fn is_registered(&self, webid: &WebID) -> bool;

    /// Revoke a capability token by ID
    async fn revoke_capability(&self, token_id: &str, holder: &WebID) -> Result<(), AcpError>;

    /// Get all capability tokens for a registered agent
    async fn get_capabilities(&self, webid: &WebID) -> Vec<DelegationToken>;

    /// List all registered agents
    async fn list_agents(&self) -> Vec<crate::acp::AcpAgent>;
}
