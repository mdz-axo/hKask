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

use hkask_types::{AuditOutcome, CapabilityAction, CapabilityResource, CapabilityToken, WebID};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::info;
use zeroize::Zeroizing;

/// Per-agent derived signing key
type AgentSecret = Arc<Zeroizing<Vec<u8>>>;

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

    #[error("Transport error: {0}")]
    TransportError(String),

    #[error("Clock error: {0}")]
    ClockError(String),

    #[error("Key derivation failed: {0}")]
    KeyDerivation(String),

    #[error(transparent)]
    Infra(#[from] hkask_types::InfrastructureError),
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
    /// Master secret for HMAC signing (Arc<Zeroizing> to avoid copying on Clone)
    secret: Arc<Zeroizing<Vec<u8>>>,
    /// Per-agent derived signing keys (HKDF-SHA256 from master key, lazily populated)
    agent_secrets: Arc<RwLock<HashMap<WebID, AgentSecret>>>,
    /// Audit log for A2A message tracking
    audit_log: Arc<AuditLog>,
    /// Root authority for OCAP capability delegation
    root_authority: Arc<RootAuthority>,
    /// Revoked capability token IDs
    revoked_tokens: Arc<RwLock<std::collections::HashSet<String>>>,
}

impl AcpRuntime {
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
            agent_secrets: Arc::new(RwLock::new(HashMap::new())),
            audit_log: Arc::new(AuditLog::new()),
            root_authority,
            revoked_tokens: Arc::new(RwLock::new(std::collections::HashSet::new())),
        }
    }

    /// Derive a per-agent signing key from the master secret using HKDF-SHA256.
    ///
    /// Each agent gets a unique key derived with `info = "hkask:acp-agent:{webid}"`.
    /// This limits blast radius: compromising one agent's key doesn't compromise
    /// others, because keys are cryptographically independent.
    ///
    /// Derived keys are cached for reuse. The master key can still derive any
    /// agent's key (root authority).
    async fn derive_agent_secret(&self, agent_webid: &WebID) -> AgentSecret {
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

    /// Resolve the signing key for a token based on its `delegated_from` field.
    ///
    /// - Root tokens (delegated_from == root authority) → master key
    /// - Delegated tokens (delegated_from == agent) → that agent's derived key
    async fn resolve_signing_key(&self, delegated_from: &WebID) -> AgentSecret {
        let root_webid = self.root_authority.root_webid();
        if *delegated_from == *root_webid {
            // Root authority tokens are signed with the master key
            Arc::clone(&self.secret)
        } else {
            // Agent-delegated tokens are signed with the agent's derived key
            self.derive_agent_secret(delegated_from).await
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

        // Create capability tokens for ALL capabilities via root authority
        let mut tokens_vec: Vec<CapabilityToken> = Vec::with_capacity(capabilities.len());
        for cap in &capabilities {
            let (resource, action) = parse_capability(cap)?;
            let token = self
                .root_authority
                .create_root_token(resource, cap.clone(), action, webid)
                .await?;
            tokens_vec.push(token);
        }

        // If no capabilities were provided, mint a default token
        let primary_token = if let Some(first) = tokens_vec.first() {
            first.clone()
        } else {
            let (resource, action) = parse_capability("tool:execute")?;
            let token = self
                .root_authority
                .create_root_token(resource, "tool:execute".to_string(), action, webid)
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
            agent_type = %agent_type,
            capabilities = ?capabilities,
            "Agent registered with ACP runtime"
        );

        Ok(primary_token)
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
    ///
    /// Uses per-agent signing keys: the key is resolved from `token.delegated_from`.
    /// Root tokens use the master key; delegated tokens use the delegating agent's
    /// derived key.
    pub async fn verify_capability(&self, token: &CapabilityToken) -> bool {
        let signing_key = self.resolve_signing_key(&token.delegated_from).await;
        let current_time = chrono::Utc::now().timestamp();
        token.verify(signing_key.as_ref()) && !token.is_expired(current_time) && {
            let revoked = self.revoked_tokens.read().await;
            !revoked.contains(&token.id)
        }
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
    /// The child token is signed with the delegating agent's derived key
    /// (the current holder of the parent token), not the master key.
    /// This limits blast radius: compromising the master key is not sufficient
    /// to forge delegated tokens.
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

        // Resolve the signing key for the delegating agent (parent token holder)
        let signing_key = self.resolve_signing_key(&parent_token.delegated_to).await;

        // Create attenuated token signed with the delegating agent's key
        let child = parent_token
            .attenuate(new_holder, signing_key.as_ref(), current_time)
            .ok_or_else(|| {
                AcpError::CapabilityDenied(
                    parent_token.delegated_to,
                    "Attenuation limit exceeded".to_string(),
                )
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
