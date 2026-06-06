//! ACP (Agent Communication Protocol) Runtime Integration
//!
//! This module provides ACP runtime adapters for agent registration,
//! A2A message handling, and capability-gated communication.
//!
//! # Security Model
//!
//! - **No hardcoded secrets**: Secret loaded from environment or keystore
//! - **Explicit capabilities**: No wildcard tokens; each capability explicitly granted
//! - **Audit logging**: All A2A messages logged for forensic analysis
//!
//! # A2A Message Flow
//!
//! ```text
//! Agent Pod A → ACP Message (template:dispatch) → hKask Router → Agent Pod B
//!                                                              ↓
//!                                                   Capability Verification
//!                                                              ↓
//!                                                   Audit Log Entry
//!                                                              ↓
//!                                                   Template Execution
//!                                                              ↓
//!                                                   Response to Agent A
//! ```

mod audit;

mod root_authority;

pub use audit::AuditEntry;
pub(crate) use audit::AuditLog;

pub use hkask_types::AuditLogPort;
pub(crate) use root_authority::RootAuthority;

use hkask_types::{
    AgentKind, AuditOutcome, CapabilitySpec, DelegationAction, DelegationResource, DelegationToken,
    WebID,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::info;
use zeroize::Zeroizing;

/// Per-agent derived signing key
pub type AgentSecret = Arc<Zeroizing<Vec<u8>>>;

/// Parse a capability string using the canonical [`CapabilitySpec`] parser.
///
/// Converts `CapabilityParseError` into `AcpError::MalformedCapability`.
fn parse_capability(
    capability: &str,
) -> Result<(DelegationResource, String, DelegationAction), AcpError> {
    let spec = CapabilitySpec::parse(capability)
        .map_err(|e| AcpError::MalformedCapability(e.to_string()))?;
    Ok((spec.resource, spec.resource_id, spec.action))
}

/// ACP error types for security and validation
#[derive(Debug, Error)]
pub enum AcpError {
    #[error("Agent {0:?} already registered")]
    AgentAlreadyRegistered(WebID),

    #[error("Agent {0:?} not found")]
    AgentNotFound(WebID),

    #[error("Capability denied: agent {0:?} lacks permission for {1}")]
    CapabilityDenied(WebID, String),

    #[error("Invalid capability: wildcards not allowed")]
    WildcardCapabilityNotAllowed,

    #[error("Malformed capability: {0}")]
    MalformedCapability(String),

    #[error("Transport error: {0}")]
    TransportError(String),

    #[error("Clock error: {0}")]
    ClockError(String),

    #[error("Key derivation failed: {0}")]
    KeyDerivation(String),

    #[error(transparent)]
    Infra(#[from] hkask_types::InfrastructureError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpAgent {
    pub webid: WebID,
    pub agent_type: AgentKind,
    /// Explicit capabilities — no wildcards
    pub capabilities: Vec<String>,
    pub registered_at: i64,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "message_type")]
pub enum A2AMessage {
    /// Template dispatch
    TemplateDispatch {
        from: WebID,
        /// Recipient (or broadcast)
        to: Option<WebID>,
        template_id: String,
        input: serde_json::Value,
        correlation_id: String,
    },
    /// Template response
    TemplateResponse {
        correlation_id: String,
        result: serde_json::Value,
        error: Option<String>,
    },
    /// Memory artifact notification
    MemoryArtifact {
        producer: WebID,
        artifact_type: String,
        artifact_id: String,
        visibility: String,
    },
}

pub struct AcpRuntime {
    agents: Arc<RwLock<HashMap<WebID, AcpAgent>>>,
    pending_messages: Arc<RwLock<HashMap<String, A2AMessage>>>,
    capability_tokens: Arc<RwLock<HashMap<WebID, Vec<DelegationToken>>>>,
    // Arc<Zeroizing> to avoid copying on Clone
    secret: Arc<Zeroizing<Vec<u8>>>,
    // HKDF-SHA256 from master key, lazily populated
    agent_secrets: Arc<RwLock<HashMap<WebID, AgentSecret>>>,
    audit_log: Arc<AuditLog>,
    root_authority: Arc<RootAuthority>,
    revoked_tokens: Arc<RwLock<std::collections::HashSet<String>>>,
}

impl AcpRuntime {
    /// `secret` is HMAC key (zeroized on drop).
    pub fn new(secret: &[u8]) -> Self {
        // Derive root WebID deterministically from a fixed "root" persona
        let root_persona = b"hkask-root-authority";
        let root_webid = WebID::from_persona(root_persona);
        let root_authority = Arc::new(RootAuthority::new(root_webid, secret));
        let secret_arc = Arc::new(Zeroizing::new(secret.to_vec()));

        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            pending_messages: Arc::new(RwLock::new(HashMap::new())),
            capability_tokens: Arc::new(RwLock::new(HashMap::new())),
            secret: secret_arc,
            agent_secrets: Arc::new(RwLock::new(HashMap::new())),
            audit_log: Arc::new(AuditLog::new()),
            root_authority,
            revoked_tokens: Arc::new(RwLock::new(std::collections::HashSet::new())),
        }
    }

    /// Keys are cryptographically independent — compromising one doesn't compromise others.
    pub async fn derive_agent_secret(&self, agent_webid: &WebID) -> AgentSecret {
        // Check cache first
        {
            let cache = self.agent_secrets.read().await;
            if let Some(key) = cache.get(agent_webid) {
                return Arc::clone(key);
            }
        }

        // Derive using HKDF-SHA256 with agent WebID as domain separator
        let context = format!("hkask:acp-agent:{}", agent_webid);
        let derived = hkask_keystore::derive_sub_key(self.secret.as_ref(), &context);
        let arc_key = Arc::new(derived);

        // Cache the derived key
        {
            let mut cache = self.agent_secrets.write().await;
            cache.insert(*agent_webid, Arc::clone(&arc_key));
        }

        arc_key
    }

    /// Returns primary DelegationToken for the agent.
    pub async fn register_agent(
        &self,
        webid: WebID,
        agent_type: AgentKind,
        capabilities: Vec<String>,
    ) -> Result<DelegationToken, AcpError> {
        let mut agents = self.agents.write().await;

        if agents.contains_key(&webid) {
            return Err(AcpError::AgentAlreadyRegistered(webid));
        }

        // Validate capabilities - reject wildcards
        for cap in &capabilities {
            if cap == "*" {
                return Err(AcpError::WildcardCapabilityNotAllowed);
            }
        }

        let agent = AcpAgent {
            webid,
            agent_type,
            capabilities: capabilities.clone(),
            registered_at: current_timestamp()?,
            active: true,
        };

        // Create delegation tokens for ALL capabilities via root authority
        let mut tokens_vec: Vec<DelegationToken> = Vec::with_capacity(capabilities.len());
        for cap in &capabilities {
            let (resource, resource_id, action) = parse_capability(cap)?;
            let token = self
                .root_authority
                .create_root_token(resource, resource_id, action, webid)
                .await?;
            tokens_vec.push(token);
        }

        // If no capabilities were provided, mint a default token
        let primary_token = if let Some(first) = tokens_vec.first() {
            first.clone()
        } else {
            let (resource, resource_id, action) = parse_capability("tool:execute")?;
            let token = self
                .root_authority
                .create_root_token(resource, resource_id, action, webid)
                .await?;
            tokens_vec.push(token.clone());
            token
        };

        // Store agent and capabilities
        agents.insert(webid, agent);

        // Store ALL capability tokens
        let mut tokens = self.capability_tokens.write().await;
        tokens.insert(webid, tokens_vec);

        info!(
            target: "hkask.acp",
            webid = %webid,
            agent_type = %agent_type.as_str(),
            capabilities = ?capabilities,
            "Agent registered with ACP runtime"
        );

        Ok(primary_token)
    }

    pub async fn unregister_agent(&self, webid: &WebID) -> Result<(), AcpError> {
        let mut agents = self.agents.write().await;

        if agents.remove(webid).is_none() {
            return Err(AcpError::AgentNotFound(*webid));
        }

        // Remove capability tokens
        let mut tokens = self.capability_tokens.write().await;
        tokens.remove(webid);

        // Remove per-agent derived key from cache
        let mut agent_secrets = self.agent_secrets.write().await;
        agent_secrets.remove(webid);

        info!(
            target: "hkask.acp",
            webid = %webid,
            "Agent unregistered from ACP runtime"
        );

        Ok(())
    }

    /// R2: Persist Agent State. Returns count of agents restored.
    pub async fn restore_from_storage(
        &self,
        agents: Vec<AcpAgent>,
        tokens: std::collections::HashMap<WebID, Vec<DelegationToken>>,
    ) -> Result<usize, AcpError> {
        let mut agents_lock = self.agents.write().await;
        let mut tokens_lock = self.capability_tokens.write().await;

        let count = agents.len();

        for agent in agents {
            agents_lock.insert(agent.webid, agent);
        }

        for (webid, token_list) in tokens {
            tokens_lock.insert(webid, token_list);
        }

        info!(
            target: "hkask.acp",
            agent_count = count,
            "Agent state restored from storage"
        );

        Ok(count)
    }

    pub(crate) async fn is_registered(&self, webid: &WebID) -> bool {
        let agents = self.agents.read().await;
        agents.contains_key(webid)
    }

    pub(crate) async fn send_message(&self, message: A2AMessage) -> Result<String, AcpError> {
        let (correlation_id, from, to, message_type) = match &message {
            A2AMessage::TemplateDispatch {
                correlation_id,
                from,
                to,
                ..
            } => (
                correlation_id.clone(),
                Some(*from),
                *to,
                "template_dispatch".to_string(),
            ),
            A2AMessage::TemplateResponse { correlation_id, .. } => (
                correlation_id.clone(),
                None,
                None,
                "template_response".to_string(),
            ),
            A2AMessage::MemoryArtifact {
                artifact_id,
                producer,
                ..
            } => (
                format!("artifact:{}", artifact_id),
                Some(*producer),
                None,
                "memory_artifact".to_string(),
            ),
        };

        let mut pending = self.pending_messages.write().await;
        pending.insert(correlation_id.clone(), message);

        // Log audit entry
        let mut audit_entry = AuditEntry::new(
            from.unwrap_or(WebID::new()),
            "sent".to_string(),
            message_type.clone(),
            AuditOutcome::Success,
        )
        .with_correlation_id(correlation_id.clone())
        .with_metadata(serde_json::json!({
            "event_type": "sent",
            "message_type": &message_type
        }));
        if let Some(recipient) = to {
            audit_entry = audit_entry.with_recipient(recipient);
        }
        self.audit_log.log(audit_entry).await;

        info!(
            target: "hkask.acp",
            correlation_id = %correlation_id,
            message_type = %message_type,
            "A2A message sent"
        );

        Ok(correlation_id)
    }

    /// Revoke a capability token by ID
    pub(crate) async fn revoke_capability(&self, token_id: &str) {
        let mut revoked = self.revoked_tokens.write().await;
        revoked.insert(token_id.to_string());
    }

    /// Get all delegation tokens for agent
    pub(crate) async fn get_capabilities(&self, webid: &WebID) -> Vec<DelegationToken> {
        let tokens = self.capability_tokens.read().await;
        tokens.get(webid).cloned().unwrap_or_default()
    }

    /// List all registered agents
    pub async fn list_agents(&self) -> Vec<AcpAgent> {
        let agents = self.agents.read().await;
        agents.values().cloned().collect()
    }
}

fn current_timestamp() -> Result<i64, AcpError> {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .map_err(|e| AcpError::ClockError(e.to_string()))
}

impl Default for AcpRuntime {
    fn default() -> Self {
        let secret = hkask_keystore::resolve_acp_secret().expect(
            "ACP secret not available. Run `kask chat` to complete onboarding, \
                 or set HKASK_MASTER_KEY or HKASK_ACP_SECRET.",
        );
        Self::new(&secret)
    }
}

#[async_trait::async_trait]
impl crate::ports::AcpPort for AcpRuntime {
    async fn register_agent(
        &self,
        webid: WebID,
        agent_type: AgentKind,
        capabilities: Vec<String>,
    ) -> Result<DelegationToken, AcpError> {
        AcpRuntime::register_agent(self, webid, agent_type, capabilities).await
    }

    async fn unregister_agent(&self, webid: &WebID) -> Result<(), AcpError> {
        AcpRuntime::unregister_agent(self, webid).await
    }

    async fn send_message(&self, msg: A2AMessage) -> Result<String, AcpError> {
        AcpRuntime::send_message(self, msg).await
    }

    async fn list_capabilities(&self, webid: &WebID) -> Result<Vec<String>, AcpError> {
        let agents = self.agents.read().await;
        agents
            .get(webid)
            .map(|agent| agent.capabilities.clone())
            .ok_or(AcpError::AgentNotFound(*webid))
    }

    async fn is_registered(&self, webid: &WebID) -> bool {
        AcpRuntime::is_registered(self, webid).await
    }

    async fn revoke_capability(&self, token_id: &str, _holder: &WebID) -> Result<(), AcpError> {
        AcpRuntime::revoke_capability(self, token_id).await;
        Ok(())
    }

    async fn get_capabilities(&self, webid: &WebID) -> Vec<DelegationToken> {
        AcpRuntime::get_capabilities(self, webid).await
    }

    async fn list_agents(&self) -> Vec<AcpAgent> {
        AcpRuntime::list_agents(self).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::id::WebID;
    use std::sync::Arc;

    #[tokio::test]
    async fn derive_agent_secret_deterministic() {
        let rt = AcpRuntime::new(b"test-master-secret");
        let webid = WebID::from_persona(b"agent-1");
        let key1 = rt.derive_agent_secret(&webid).await;
        let key2 = rt.derive_agent_secret(&webid).await;
        assert_eq!(key1.as_ref().as_slice(), key2.as_ref().as_slice());
    }

    #[tokio::test]
    async fn derive_agent_secret_different_agents_different_keys() {
        let rt = AcpRuntime::new(b"test-master-secret");
        let key1 = rt
            .derive_agent_secret(&WebID::from_persona(b"agent-1"))
            .await;
        let key2 = rt
            .derive_agent_secret(&WebID::from_persona(b"agent-2"))
            .await;
        assert!(key1.as_ref().as_slice() != key2.as_ref().as_slice());
    }

    #[tokio::test]
    async fn derive_agent_secret_different_masters_different_keys() {
        let rt1 = AcpRuntime::new(b"master-secret-a");
        let rt2 = AcpRuntime::new(b"master-secret-b");
        let webid = WebID::from_persona(b"agent-1");
        let key1 = rt1.derive_agent_secret(&webid).await;
        let key2 = rt2.derive_agent_secret(&webid).await;
        assert!(key1.as_ref().as_slice() != key2.as_ref().as_slice());
    }

    #[tokio::test]
    async fn derive_agent_secret_caches_keys() {
        let rt = AcpRuntime::new(b"test-master-secret");
        let webid = WebID::from_persona(b"agent-1");
        let key1 = rt.derive_agent_secret(&webid).await;
        let key2 = rt.derive_agent_secret(&webid).await;
        assert!(Arc::ptr_eq(&key1, &key2));
    }
}
