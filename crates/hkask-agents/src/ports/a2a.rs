//! A2A Port — Agent Communication Protocol hexagonal port
//!
//! Defines the interface for agent registration, A2A messaging,
//! and capability management.

use hkask_capability::DelegationToken;
use hkask_types::{AgentKind, WebID};

use crate::a2a::{A2AError, A2AMessage};

/// A2A Port — Agent registration and A2A communication
///
/// # Hexagonal Architecture
///
/// This port is implemented by `A2ARuntime` (in-process) and can be
/// adapted for remote A2A servers via transport adapters.

#[async_trait::async_trait]
pub trait A2APort: Send + Sync {
    async fn register_agent(
        &self,
        webid: WebID,
        agent_type: AgentKind,
        capabilities: Vec<String>,
    ) -> Result<DelegationToken, A2AError>;

    async fn unregister_agent(&self, webid: &WebID) -> Result<(), A2AError>;

    async fn send_message(&self, msg: A2AMessage) -> Result<String, A2AError>;

    async fn list_capabilities(&self, webid: &WebID) -> Result<Vec<String>, A2AError>;

    async fn is_registered(&self, webid: &WebID) -> bool;

    /// Revoke a capability token by ID
    async fn revoke_capability(&self, token_id: &str, holder: &WebID) -> Result<(), A2AError>;

    /// Get all capability tokens for a registered agent
    async fn get_capabilities(&self, webid: &WebID) -> Vec<DelegationToken>;

    /// List all registered agents
    async fn list_agents(&self) -> Vec<crate::a2a::A2AAgent>;
}
