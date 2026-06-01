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

pub use audit::{AuditEntry, AuditLog};

pub use hkask_types::AuditLogPort;
pub use root_authority::RootAuthority;

use hkask_types::{AuditOutcome, CapabilityAction, CapabilityResource, CapabilityToken, WebID};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::info;
use zeroize::Zeroizing;

/// Parse a capability string into resource and action
///
/// Examples:
/// - "tool:execute" -> (CapabilityResource::Tool, CapabilityAction::Execute)
/// - "template:render" -> (CapabilityResource::Template, CapabilityAction::Render)
///
/// # Errors
/// Returns `AcpError::MalformedCapability` if the capability string is invalid
fn parse_capability(capability: &str) -> Result<(CapabilityResource, CapabilityAction), AcpError> {
    let parts: Vec<&str> = capability.split(':').collect();

    if parts.len() < 2 || parts.len() > 3 {
        return Err(AcpError::MalformedCapability(format!(
            "Expected format 'resource:action' or 'resource:domain:action', got '{}'",
            capability
        )));
    }

    let resource = CapabilityResource::parse_str(parts[0]).ok_or_else(|| {
        AcpError::MalformedCapability(format!("Unknown resource type: {}", parts[0]))
    })?;

    let action_str = parts
        .last()
        .expect("splitn always produces at least one element");
    let action = CapabilityAction::parse_str(action_str).unwrap_or(CapabilityAction::Execute);

    Ok((resource, action))
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

    #[error("Message correlation ID not found: {0}")]
    CorrelationIdNotFound(String),

    #[error("Secret not configured: set HKASK_ACP_SECRET environment variable")]
    SecretNotConfigured,

    #[error("Invalid attenuation chain: {0}")]
    InvalidAttenuationChain(String),

    #[error("Transport error: {0}")]
    TransportError(String),

    #[error("Non-loopback address refused: {0}")]
    NonLoopbackRefused(std::net::IpAddr),

    #[error("Connection refused: {0}")]
    ConnectionRefused(String),

    #[error("Transport disconnected")]
    Disconnected,

    #[error("Clock error: {0}")]
    ClockError(String),

    #[error("Key derivation failed: {0}")]
    KeyDerivation(String),

    #[error("{0}")]
    LegacyError(String),

    #[error(transparent)]
    Infra(#[from] hkask_types::InfrastructureError),
}

impl From<String> for AcpError {
    fn from(s: String) -> Self {
        AcpError::LegacyError(s)
    }
}

/// ACP agent registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpAgent {
    /// Agent WebID
    pub webid: WebID,
    /// Agent type (Bot or Replicant)
    pub agent_type: String,
    /// Registered capabilities (explicit, no wildcards)
    pub capabilities: Vec<String>,
    /// Registration timestamp (Unix epoch)
    pub registered_at: i64,
    /// Active status
    pub active: bool,
}

/// A2A message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "message_type")]
pub enum A2AMessage {
    /// Template dispatch request
    TemplateDispatch {
        /// Sender WebID
        from: WebID,
        /// Recipient WebID (or broadcast)
        to: Option<WebID>,
        /// Template ID to invoke
        template_id: String,
        /// Input data
        input: serde_json::Value,
        /// Correlation ID for response matching
        correlation_id: String,
    },
    /// Template dispatch response
    TemplateResponse {
        /// Original correlation ID
        correlation_id: String,
        /// Response data
        result: serde_json::Value,
        /// Error message (if any)
        error: Option<String>,
    },
    /// Memory artifact notification
    MemoryArtifact {
        /// Producer WebID
        producer: WebID,
        /// Artifact type
        artifact_type: String,
        /// Artifact ID
        artifact_id: String,
        /// Visibility setting
        visibility: String,
    },
}

pub struct AcpRuntime {
    /// Registered agents
    agents: Arc<RwLock<HashMap<WebID, AcpAgent>>>,
    /// Pending messages (correlation_id -> message)
    pending_messages: Arc<RwLock<HashMap<String, A2AMessage>>>,
    /// Capability tokens indexed by holder WebID
    capability_tokens: Arc<RwLock<HashMap<WebID, Vec<CapabilityToken>>>>,
    /// Secret for HMAC signing (Arc<Zeroizing> to avoid copying on Clone)
    secret: Arc<Zeroizing<Vec<u8>>>,
    /// Audit log for A2A message tracking
    audit_log: Arc<AuditLog>,
    /// Root authority for OCAP capability delegation
    root_authority: Arc<RootAuthority>,
    /// Revoked capability token IDs
    revoked_tokens: Arc<RwLock<std::collections::HashSet<String>>>,
}

impl AcpRuntime {
    /// Create new ACP runtime with secret from environment
    ///
    /// # Security
    ///
    /// Secret is loaded from `HKASK_ACP_SECRET` environment variable.
    /// If not set, returns `AcpError::SecretNotConfigured`.
    ///
    /// # Returns
    ///
    /// * `Ok(AcpRuntime)` - Runtime initialized successfully
    /// * `Err(AcpError::SecretNotConfigured)` - Environment variable not set
    pub fn from_env() -> Result<Self, AcpError> {
        let secret_str =
            std::env::var("HKASK_ACP_SECRET").map_err(|_| AcpError::SecretNotConfigured)?;

        let secret = Arc::new(Zeroizing::new(secret_str.into_bytes()));
        let audit_log = Arc::new(AuditLog::new());
        let root_webid = WebID::new();
        let root_authority = Arc::new(RootAuthority::new(root_webid, &secret));

        Ok(Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            pending_messages: Arc::new(RwLock::new(HashMap::new())),
            capability_tokens: Arc::new(RwLock::new(HashMap::new())),
            secret,
            audit_log,
            root_authority,
            revoked_tokens: Arc::new(RwLock::new(std::collections::HashSet::new())),
        })
    }

    /// Create new ACP runtime with explicit secret
    ///
    /// # Arguments
    ///
    /// * `secret` - HMAC secret key (will be zeroized on drop)
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
            audit_log: Arc::new(AuditLog::new()),
            root_authority,
            revoked_tokens: Arc::new(RwLock::new(std::collections::HashSet::new())),
        }
    }

    /// Register an agent with the ACP runtime
    ///
    /// # Arguments
    /// * `webid` — Agent's WebID
    /// * `agent_type` — "Bot" or "Replicant"
    /// * `capabilities` — List of capability strings
    ///
    /// # Returns
    /// * `Ok(CapabilityToken)` — Primary capability token for the agent
    /// * `Err(AcpError)` — Registration error
    pub async fn register_agent(
        &self,
        webid: WebID,
        agent_type: String,
        capabilities: Vec<String>,
    ) -> Result<CapabilityToken, AcpError> {
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
            agent_type: agent_type.clone(),
            capabilities: capabilities.clone(),
            registered_at: current_timestamp()?,
            active: true,
        };

        // Create primary capability token via root authority
        let primary_capability = capabilities
            .first()
            .cloned()
            .unwrap_or_else(|| "tool:execute".to_string());

        let (resource, action) = parse_capability(&primary_capability)?;

        let token = self
            .root_authority
            .create_root_token(resource, primary_capability.clone(), action, webid)
            .await?;

        // Store agent and capabilities
        agents.insert(webid, agent);

        // Store capability token
        let mut tokens = self.capability_tokens.write().await;
        tokens
            .entry(webid)
            .or_insert_with(Vec::new)
            .push(token.clone());

        info!(
            target: "hkask.acp",
            webid = %webid,
            agent_type = %agent_type,
            capabilities = ?capabilities,
            "Agent registered with ACP runtime"
        );

        Ok(token)
    }

    /// Unregister an agent
    pub async fn unregister_agent(&self, webid: &WebID) -> Result<(), AcpError> {
        let mut agents = self.agents.write().await;

        if agents.remove(webid).is_none() {
            return Err(AcpError::AgentNotFound(*webid));
        }

        // Remove capability tokens
        let mut tokens = self.capability_tokens.write().await;
        tokens.remove(webid);

        info!(
            target: "hkask.acp",
            webid = %webid,
            "Agent unregistered from ACP runtime"
        );

        Ok(())
    }

    /// Restore agent state from storage (R2: Persist Agent State)
    ///
    /// # Arguments
    /// * `agents` - List of registered agents to restore
    /// * `tokens` - Map of WebID to capability tokens
    ///
    /// # Returns
    /// * `Ok(usize)` - Number of agents restored
    /// * `Err(AcpError)` - Restoration error
    pub async fn restore_from_storage(
        &self,
        agents: Vec<AcpAgent>,
        tokens: std::collections::HashMap<WebID, Vec<CapabilityToken>>,
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

    /// Get agent by WebID
    pub async fn get_agent(&self, webid: &WebID) -> Option<AcpAgent> {
        let agents = self.agents.read().await;
        agents.get(webid).cloned()
    }

    /// Check if agent is registered
    pub async fn is_registered(&self, webid: &WebID) -> bool {
        let agents = self.agents.read().await;
        agents.contains_key(webid)
    }

    /// Send A2A message
    pub async fn send_message(&self, message: A2AMessage) -> Result<String, AcpError> {
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

    /// Get pending message by correlation ID
    pub async fn get_message(&self, correlation_id: &str) -> Option<A2AMessage> {
        let pending = self.pending_messages.read().await;
        pending.get(correlation_id).cloned()
    }

    /// Remove pending message
    pub async fn remove_message(&self, correlation_id: &str) -> Option<A2AMessage> {
        let mut pending = self.pending_messages.write().await;
        pending.remove(correlation_id)
    }

    /// Verify capability token (HMAC signature + revocation check + expiry check)
    pub async fn verify_capability(&self, token: &CapabilityToken) -> bool {
        let current_time = chrono::Utc::now().timestamp();
        let valid = token.verify(self.secret.as_ref()) && !token.is_expired(current_time) && {
            let revoked = self.revoked_tokens.read().await;
            !revoked.contains(&token.id)
        };

        valid
    }

    /// Revoke a capability token by ID
    pub async fn revoke_capability(&self, token_id: &str) {
        let mut revoked = self.revoked_tokens.write().await;
        revoked.insert(token_id.to_string());
    }

    /// Check if a capability token has been revoked
    pub async fn is_revoked(&self, token_id: &str) -> bool {
        let revoked = self.revoked_tokens.read().await;
        revoked.contains(token_id)
    }

    /// Delegate capability to another agent
    ///
    /// Creates an attenuated child token from the parent token.
    /// The child token has reduced authority (attenuation_level + 1).
    ///
    /// # Arguments
    /// * `parent_token` — Parent capability token
    /// * `new_holder` — WebID of the delegate
    /// * `current_time` — Current Unix timestamp for expiry
    ///
    /// # Returns
    /// * `Ok(CapabilityToken)` — Attenuated child token
    /// * `Err(AcpError)` — Delegation failed (attenuation limit, etc.)
    pub async fn delegate_capability(
        &self,
        parent_token: &CapabilityToken,
        new_holder: WebID,
        current_time: i64,
    ) -> Result<CapabilityToken, AcpError> {
        // Verify parent token is valid
        if !self.verify_capability(parent_token).await {
            return Err(AcpError::CapabilityDenied(
                parent_token.delegated_to,
                "Invalid parent token signature".to_string(),
            ));
        }

        // Verify attenuation chain
        self.root_authority
            .verify_attenuation_chain(parent_token, self.root_authority.root_webid())?;

        // Create attenuated token
        let child = parent_token
            .attenuate(new_holder, self.secret.as_ref(), current_time)
            .ok_or_else(|| {
                AcpError::InvalidAttenuationChain("Attenuation limit exceeded".to_string())
            })?;

        Ok(child)
    }

    /// Verify capability attenuation chain
    ///
    /// Ensures the token traces back to the root authority
    /// and the attenuation chain is unbroken.
    pub async fn verify_capability_chain(&self, token: &CapabilityToken) -> Result<(), AcpError> {
        if !self.verify_capability(token).await {
            return Err(AcpError::CapabilityDenied(
                token.delegated_to,
                "Invalid token signature".to_string(),
            ));
        }

        self.root_authority
            .verify_attenuation_chain(token, self.root_authority.root_webid())
    }

    /// Store capability token for agent
    pub async fn store_capability(&self, webid: WebID, token: CapabilityToken) {
        let mut tokens = self.capability_tokens.write().await;
        tokens.entry(webid).or_insert_with(Vec::new).push(token);
    }

    /// Get all capability tokens for agent
    pub async fn get_capabilities(&self, webid: &WebID) -> Vec<CapabilityToken> {
        let tokens = self.capability_tokens.read().await;
        tokens.get(webid).cloned().unwrap_or_default()
    }

    /// Check if agent has capability for tool
    pub async fn has_capability(&self, webid: &WebID, capability: &str) -> bool {
        let agents = self.agents.read().await;
        if let Some(agent) = agents.get(webid) {
            // Check if agent has the exact capability registered (no wildcards)
            agent.capabilities.iter().any(|cap| cap == capability)
        } else {
            false
        }
    }

    /// List all registered agents
    pub async fn list_agents(&self) -> Vec<AcpAgent> {
        let agents = self.agents.read().await;
        agents.values().cloned().collect()
    }

    /// Get agent count
    pub async fn agent_count(&self) -> usize {
        let agents = self.agents.read().await;
        agents.len()
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
        let secret = hkask_keystore::resolve(&hkask_types::SecretRef::derived(
            hkask_types::derivation_contexts::MASTER_KEY_ENV,
            hkask_types::derivation_contexts::ACP_SECRET,
        ))
        .or_else(|_| hkask_keystore::resolve(&hkask_types::SecretRef::env("HKASK_ACP_SECRET_KEY")))
        .or_else(|_| {
            hkask_keystore::resolve(&hkask_types::SecretRef::Keychain("acp-secret".to_string()))
        })
        .expect(
            "ACP secret not available. Run `kask chat` to complete onboarding, \
             or set HKASK_MASTER_KEY or HKASK_ACP_SECRET_KEY.",
        );
        Self::new(&secret)
    }
}

#[async_trait::async_trait]
impl crate::ports::AcpPort for AcpRuntime {
    async fn register_agent(
        &self,
        webid: WebID,
        agent_type: &str,
        capabilities: Vec<String>,
    ) -> Result<CapabilityToken, AcpError> {
        AcpRuntime::register_agent(self, webid, agent_type.to_string(), capabilities).await
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

    async fn get_capabilities(&self, webid: &WebID) -> Vec<CapabilityToken> {
        AcpRuntime::get_capabilities(self, webid).await
    }

    async fn list_agents(&self) -> Vec<AcpAgent> {
        AcpRuntime::list_agents(self).await
    }
}
