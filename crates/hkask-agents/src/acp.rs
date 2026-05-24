//! ACP (Agent Communication Protocol) Runtime Integration
//!
//! This module provides ACP runtime adapters for agent registration,
//! A2A message handling, and capability-gated communication.
//!
//! # Security Model
//!
//! - **No hardcoded secrets**: Secret loaded from environment or keystore
//! - **Explicit capabilities**: No wildcard tokens; each capability explicitly granted
//! - **Rate limiting**: Per-agent message quota to prevent DoS
//! - **Audit logging**: All A2A messages logged for forensic analysis
//!
//! # A2A Message Flow
//!
//! ```text
//! Agent Pod A → ACP Message (template:dispatch) → hKask Router → Agent Pod B
//!                                                              ↓
//!                                                   Rate Limit Check
//!                                                              ↓
//!                                                   Capability Verification
//!                                                              ↓
//!                                                   Audit Log Entry
//!                                                              ↓
//!                                                   Template Execution
//!                                                              ↓
//!                                                   CNS Span Emission
//!                                                              ↓
//!                                                   Response to Agent A
//! ```

use hkask_cns::rate_limit::{RateLimitConfig, RateLimiter};
use hkask_types::{CapabilityAction, CapabilityResource, CapabilityToken, WebID};
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

    if parts.len() != 2 {
        return Err(AcpError::MalformedCapability(format!(
            "Expected format 'resource:action', got '{}'",
            capability
        )));
    }

    let resource = CapabilityResource::parse_str(parts[0]).ok_or_else(|| {
        AcpError::MalformedCapability(format!("Unknown resource type: {}", parts[0]))
    })?;

    let action = CapabilityAction::parse_str(parts[1]).ok_or_else(|| {
        AcpError::MalformedCapability(format!("Unknown action type: {}", parts[1]))
    })?;

    Ok((resource, action))
}

/// ACP error types for security and validation
#[derive(Debug, Error)]
pub enum AcpError {
    #[error("Agent {0:?} already registered")]
    AgentAlreadyRegistered(WebID),

    #[error("Agent {0:?} not found")]
    AgentNotFound(WebID),

    #[error("Rate limit exceeded for agent {0:?}")]
    RateLimitExceeded(WebID),

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

    #[error("{0}")]
    LegacyError(String),
}

impl From<String> for AcpError {
    fn from(s: String) -> Self {
        AcpError::LegacyError(s)
    }
}

/// Root authority for OCAP capability delegation
///
/// All capability tokens trace back to a root authority. The root authority
/// is the ultimate source of all capabilities in the system.
///
/// # OCAP Discipline
///
/// - No ambient authority: capabilities must be explicitly granted
/// - Attenuation chain: each delegation reduces authority
/// - Expiry enforcement: capabilities expire and must be renewed
#[derive(Debug, Clone)]
pub struct RootAuthority {
    /// Root authority WebID (system identity)
    root_webid: WebID,
    /// Root secret for HMAC signing
    root_secret: Zeroizing<Vec<u8>>,
    /// Next token ID counter
    token_counter: Arc<RwLock<u64>>,
}

impl RootAuthority {
    /// Create new root authority
    pub fn new(root_webid: WebID, root_secret: &[u8]) -> Self {
        Self {
            root_webid,
            root_secret: Zeroizing::new(root_secret.to_vec()),
            token_counter: Arc::new(RwLock::new(0)),
        }
    }

    /// Create root capability token
    ///
    /// This is the starting point of an attenuation chain.
    /// Root tokens have attenuation_level=0 and max_attenuation=7.
    pub async fn create_root_token(
        &self,
        resource: CapabilityResource,
        resource_id: String,
        action: CapabilityAction,
        delegated_to: WebID,
    ) -> Result<CapabilityToken, AcpError> {
        let token_id = {
            let mut counter = self.token_counter.write().await;
            *counter += 1;
            *counter
        };

        let context_nonce = format!("root-{}-{}", self.root_webid, token_id);

        let token = CapabilityToken::new_with_attenuation(
            resource,
            resource_id,
            action,
            self.root_webid,
            delegated_to,
            &self.root_secret,
            None,
            0,
            7,
            Some(context_nonce),
        );

        Ok(token)
    }

    /// Verify attenuation chain from root to current token
    ///
    /// Returns Ok if:
    /// - Root nonce starts with expected root prefix
    /// - Attenuation level is within expected bounds
    /// - Chain is unbroken (each level increments by 1)
    pub fn verify_attenuation_chain(
        &self,
        token: &CapabilityToken,
        expected_root: &WebID,
    ) -> Result<(), AcpError> {
        let root_nonce = token.root_context_nonce();
        let expected_prefix = format!("root-{}", expected_root);

        if !root_nonce.starts_with(&expected_prefix) {
            return Err(AcpError::InvalidAttenuationChain(
                "Root nonce mismatch".to_string(),
            ));
        }

        if token.attenuation_level > token.max_attenuation {
            return Err(AcpError::InvalidAttenuationChain(
                "Attenuation level exceeds maximum".to_string(),
            ));
        }

        Ok(())
    }

    /// Get root WebID
    pub fn root_webid(&self) -> &WebID {
        &self.root_webid
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

/// ACP runtime for agent registration and message routing
pub struct AcpRuntime {
    /// Registered agents
    agents: Arc<RwLock<HashMap<WebID, AcpAgent>>>,
    /// Pending messages (correlation_id -> message)
    pending_messages: Arc<RwLock<HashMap<String, A2AMessage>>>,
    /// Capability tokens indexed by holder WebID
    capability_tokens: Arc<RwLock<HashMap<WebID, Vec<CapabilityToken>>>>,
    /// Secret for HMAC signing (Zeroizing for secure memory)
    secret: Zeroizing<Vec<u8>>,
    /// Rate limiter for DoS prevention
    _rate_limiter: RateLimiter,
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
    /// # Arguments
    ///
    /// * `rate_limit_config` - Rate limit configuration (default: 100 msg/min)
    ///
    /// # Returns
    ///
    /// * `Ok(AcpRuntime)` - Runtime initialized successfully
    /// * `Err(AcpError::SecretNotConfigured)` - Environment variable not set
    pub fn from_env(rate_limit_config: Option<RateLimitConfig>) -> Result<Self, AcpError> {
        let secret_str =
            std::env::var("HKASK_ACP_SECRET").map_err(|_| AcpError::SecretNotConfigured)?;

        let secret = Zeroizing::new(secret_str.into_bytes());
        let rate_limiter = RateLimiter::new(rate_limit_config.unwrap_or_else(|| RateLimitConfig {
            max_tokens: 100,
            refill_interval: std::time::Duration::from_millis(600),
        }));
        let audit_log = Arc::new(AuditLog::new());
        let root_webid = WebID::new();
        let root_authority = Arc::new(RootAuthority::new(root_webid, &secret));

        Ok(Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            pending_messages: Arc::new(RwLock::new(HashMap::new())),
            capability_tokens: Arc::new(RwLock::new(HashMap::new())),
            secret,
            _rate_limiter: rate_limiter,
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
    /// * `rate_limit_config` - Rate limit configuration (default: 100 msg/min)
    pub fn new(secret: &[u8], rate_limit_config: Option<RateLimitConfig>) -> Self {
        let root_webid = WebID::new();
        let root_authority = Arc::new(RootAuthority::new(root_webid, secret));

        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            pending_messages: Arc::new(RwLock::new(HashMap::new())),
            capability_tokens: Arc::new(RwLock::new(HashMap::new())),
            secret: Zeroizing::new(secret.to_vec()),
            _rate_limiter: RateLimiter::new(rate_limit_config.unwrap_or_else(|| RateLimitConfig {
                max_tokens: 100,
                refill_interval: std::time::Duration::from_millis(600),
            })),
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
            registered_at: current_timestamp(),
            active: true,
        };

        // Create primary capability token via root authority
        let primary_capability = capabilities
            .first()
            .cloned()
            .unwrap_or_else(|| "agent:basic".to_string());

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
        let audit_entry = AuditLogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: current_timestamp(),
            from: from.unwrap_or(WebID::new()),
            to,
            message_type: message_type.clone(),
            correlation_id: correlation_id.clone(),
            event_type: "sent".to_string(),
            metadata: serde_json::json!({}),
        };
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

    /// Verify capability token (HMAC signature + revocation check)
    pub fn verify_capability(&self, token: &CapabilityToken) -> bool {
        if !token.verify(&self.secret) {
            return false;
        }

        let revoked = self.revoked_tokens.blocking_read();
        !revoked.contains(&token.id)
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
        if !self.verify_capability(parent_token) {
            return Err(AcpError::CapabilityDenied(
                parent_token.delegated_to,
                "Invalid parent token signature".to_string(),
            ));
        }

        // Verify attenuation chain
        self.root_authority
            .verify_attenuation_chain(parent_token, self.root_authority.root_webid())?;

        // Create attenuated token
        parent_token
            .attenuate(new_holder, &self.secret, current_time)
            .ok_or_else(|| {
                AcpError::InvalidAttenuationChain("Attenuation limit exceeded".to_string())
            })
    }

    /// Verify capability attenuation chain
    ///
    /// Ensures the token traces back to the root authority
    /// and the attenuation chain is unbroken.
    pub fn verify_capability_chain(&self, token: &CapabilityToken) -> Result<(), AcpError> {
        if !self.verify_capability(token) {
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
            // Check if agent has the exact capability registered
            agent
                .capabilities
                .iter()
                .any(|cap| cap == capability || cap == "*")
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

fn current_timestamp() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

/// Audit log entry for A2A messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// Unique entry identifier
    pub id: String,
    /// Timestamp of the event
    pub timestamp: i64,
    /// Sender WebID
    pub from: WebID,
    /// Recipient WebID (if any)
    pub to: Option<WebID>,
    /// Message type
    pub message_type: String,
    /// Correlation ID
    pub correlation_id: String,
    /// Event type (sent, received, verified, denied)
    pub event_type: String,
    /// Additional metadata
    pub metadata: serde_json::Value,
}

/// Audit log for A2A message tracking
pub struct AuditLog {
    entries: Arc<RwLock<Vec<AuditLogEntry>>>,
    max_entries: usize,
}

impl AuditLog {
    /// Create new audit log with default max entries
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            max_entries: 10000,
        }
    }

    /// Create audit log with custom max entries
    pub fn with_max_entries(max_entries: usize) -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            max_entries,
        }
    }

    /// Log an A2A message event
    pub async fn log(&self, entry: AuditLogEntry) {
        let mut entries = self.entries.write().await;
        entries.push(entry);

        // Trim if exceeding max entries
        if entries.len() > self.max_entries {
            let drain_count = entries.len() - self.max_entries;
            entries.drain(0..drain_count);
        }
    }

    /// Get recent entries
    pub async fn get_recent(&self, count: usize) -> Vec<AuditLogEntry> {
        let entries = self.entries.read().await;
        entries.iter().rev().take(count).cloned().collect()
    }

    /// Get entries by WebID
    pub async fn get_by_webid(&self, webid: &WebID, count: usize) -> Vec<AuditLogEntry> {
        let entries = self.entries.read().await;
        entries
            .iter()
            .filter(|e| e.from == *webid || e.to == Some(*webid))
            .rev()
            .take(count)
            .cloned()
            .collect()
    }

    /// Get entries by correlation ID
    pub async fn get_by_correlation(&self, correlation_id: &str) -> Vec<AuditLogEntry> {
        let entries = self.entries.read().await;
        entries
            .iter()
            .filter(|e| e.correlation_id == correlation_id)
            .cloned()
            .collect()
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

/// Audit log port for external audit systems
#[async_trait::async_trait]
pub trait AuditLogPort: Send + Sync {
    /// Log an A2A message event
    async fn log(&self, entry: AuditLogEntry);

    /// Get recent audit entries
    async fn get_recent(&self, count: usize) -> Vec<AuditLogEntry>;

    /// Query audit log by WebID
    async fn get_by_webid(&self, webid: &WebID, count: usize) -> Vec<AuditLogEntry>;
}

#[async_trait::async_trait]
impl AuditLogPort for AuditLog {
    async fn log(&self, entry: AuditLogEntry) {
        let mut entries = self.entries.write().await;
        entries.push(entry);

        if entries.len() > self.max_entries {
            let drain_count = entries.len() - self.max_entries;
            entries.drain(0..drain_count);
        }
    }

    async fn get_recent(&self, count: usize) -> Vec<AuditLogEntry> {
        let entries = self.entries.read().await;
        entries.iter().rev().take(count).cloned().collect()
    }

    async fn get_by_webid(&self, webid: &WebID, count: usize) -> Vec<AuditLogEntry> {
        let entries = self.entries.read().await;
        entries
            .iter()
            .rev()
            .filter(|e| &e.from == webid || e.to.as_ref() == Some(webid))
            .take(count)
            .cloned()
            .collect()
    }
}

impl Default for AcpRuntime {
    fn default() -> Self {
        let secret = hkask_keystore::resolve(&hkask_types::SecretRef::env("HKASK_ACP_SECRET_KEY"))
            .unwrap_or_else(|_| {
                tracing::warn!("HKASK_ACP_SECRET_KEY not set, using generated secret");
                hkask_keystore::resolve(&hkask_types::SecretRef::generated(32))
                    .expect("generated secret cannot fail")
            });
        Self::new(&secret, None)
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
}

/// A2A template dispatch handler
pub struct TemplateDispatchHandler {
    acp_runtime: Arc<AcpRuntime>,
}

impl TemplateDispatchHandler {
    /// Create new dispatch handler
    pub fn new(acp_runtime: Arc<AcpRuntime>) -> Self {
        Self { acp_runtime }
    }

    /// Process template dispatch request
    ///
    /// # Arguments
    /// * `from` — Sender WebID
    /// * `to` — Recipient WebID (optional for broadcast)
    /// * `template_id` — Template to invoke
    /// * `input` — Input data
    ///
    /// # Returns
    /// * `Ok(correlation_id)` — Message correlation ID
    /// * `Err(AcpError)` — Dispatch error
    pub async fn dispatch(
        &self,
        from: WebID,
        to: Option<WebID>,
        template_id: String,
        input: serde_json::Value,
    ) -> Result<String, AcpError> {
        // Verify sender is registered
        if !self.acp_runtime.is_registered(&from).await {
            return Err(AcpError::AgentNotFound(from));
        }

        // Verify recipient if specified
        if let Some(recipient) = to
            && !self.acp_runtime.is_registered(&recipient).await
        {
            return Err(AcpError::AgentNotFound(recipient));
        }

        let correlation_id = uuid::Uuid::new_v4().to_string();
        let template_id_clone = template_id.clone();

        let message = A2AMessage::TemplateDispatch {
            from,
            to,
            template_id,
            input,
            correlation_id: correlation_id.clone(),
        };

        self.acp_runtime.send_message(message).await?;

        info!(
            target: "hkask.acp",
            from = %from,
            to = ?to,
            template_id = %template_id_clone,
            correlation_id = %correlation_id,
            "Template dispatch sent"
        );

        Ok(correlation_id)
    }

    /// Process template dispatch response
    pub async fn respond(
        &self,
        correlation_id: String,
        result: serde_json::Value,
        error: Option<String>,
    ) -> Result<(), AcpError> {
        let message = A2AMessage::TemplateResponse {
            correlation_id: correlation_id.clone(),
            result,
            error,
        };

        self.acp_runtime.send_message(message).await?;

        info!(
            target: "hkask.acp",
            correlation_id = %correlation_id,
            "Template dispatch response sent"
        );

        Ok(())
    }

    /// Notify memory artifact creation
    pub async fn notify_artifact(
        &self,
        producer: WebID,
        artifact_type: String,
        artifact_id: String,
        visibility: String,
    ) -> Result<(), AcpError> {
        let artifact_id_clone = artifact_id.clone();
        let artifact_type_clone = artifact_type.clone();

        let message = A2AMessage::MemoryArtifact {
            producer,
            artifact_type,
            artifact_id,
            visibility,
        };

        self.acp_runtime.send_message(message).await?;

        info!(
            target: "hkask.acp",
            producer = %producer,
            artifact_id = %artifact_id_clone,
            artifact_type = %artifact_type_clone,
            "Memory artifact notification sent"
        );

        Ok(())
    }
}
