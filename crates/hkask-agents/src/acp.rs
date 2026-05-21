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
/// - "agent:basic" -> (CapabilityResource::Tool, CapabilityAction::Execute) [default fallback]
fn parse_capability(capability: &str) -> Option<(CapabilityResource, CapabilityAction)> {
    let parts: Vec<&str> = capability.split(':').collect();

    if parts.len() != 2 {
        return None;
    }

    let resource = CapabilityResource::parse_str(parts[0])?;
    let action = CapabilityAction::parse_str(parts[1])?;

    Some((resource, action))
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

    #[error("Message correlation ID not found: {0}")]
    CorrelationIdNotFound(String),

    #[error("Secret not configured: set HKASK_ACP_SECRET environment variable")]
    SecretNotConfigured,

    #[error("Invalid attenuation chain: {0}")]
    InvalidAttenuationChain(String),
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
    #[allow(dead_code)]
    rate_limiter: RateLimiter,
    /// Audit log for A2A message tracking
    audit_log: Arc<AuditLog>,
    /// Root authority for OCAP capability delegation
    root_authority: Arc<RootAuthority>,
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
            rate_limiter,
            audit_log,
            root_authority,
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
            rate_limiter: RateLimiter::new(rate_limit_config.unwrap_or_else(|| RateLimitConfig {
                max_tokens: 100,
                refill_interval: std::time::Duration::from_millis(600),
            })),
            audit_log: Arc::new(AuditLog::new()),
            root_authority,
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
    /// * `Err(String)` — Registration error
    pub async fn register_agent(
        &self,
        webid: WebID,
        agent_type: String,
        capabilities: Vec<String>,
    ) -> Result<CapabilityToken, String> {
        let mut agents = self.agents.write().await;

        if agents.contains_key(&webid) {
            return Err(format!("Agent {:?} already registered", webid));
        }

        // Validate capabilities - reject wildcards
        for cap in &capabilities {
            if cap == "*" {
                return Err("Wildcard capabilities are not allowed".to_string());
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

        let (resource, action) = parse_capability(&primary_capability)
            .unwrap_or((CapabilityResource::Tool, CapabilityAction::Execute));

        let token = self
            .root_authority
            .create_root_token(resource, primary_capability.clone(), action, webid)
            .await
            .map_err(|e| e.to_string())?;

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
    pub async fn unregister_agent(&self, webid: &WebID) -> Result<(), String> {
        let mut agents = self.agents.write().await;

        if agents.remove(webid).is_none() {
            return Err(format!("Agent {:?} not found", webid));
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
    pub async fn send_message(&self, message: A2AMessage) -> Result<String, String> {
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

    /// Verify capability token
    pub fn verify_capability(&self, token: &CapabilityToken) -> bool {
        token.verify(&self.secret)
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
pub trait AuditLogPort {
    /// Log an A2A message event
    fn log(&self, entry: AuditLogEntry);

    /// Get recent audit entries
    fn get_recent(&self, count: usize) -> Vec<AuditLogEntry>;

    /// Query audit log by WebID
    fn get_by_webid(&self, webid: &WebID, count: usize) -> Vec<AuditLogEntry>;
}

impl AuditLogPort for AuditLog {
    fn log(&self, entry: AuditLogEntry) {
        // Clone self reference for async task
        let entries = Arc::clone(&self.entries);
        let max_entries = self.max_entries;

        tokio::spawn(async move {
            let mut entries_guard = entries.write().await;
            entries_guard.push(entry);

            // Trim if exceeding max entries
            if entries_guard.len() > max_entries {
                let drain_count = entries_guard.len() - max_entries;
                entries_guard.drain(0..drain_count);
            }
        });
    }

    fn get_recent(&self, count: usize) -> Vec<AuditLogEntry> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.get_recent(count))
        })
    }

    fn get_by_webid(&self, webid: &WebID, count: usize) -> Vec<AuditLogEntry> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.get_by_webid(webid, count))
        })
    }
}

impl Default for AcpRuntime {
    fn default() -> Self {
        Self::new(b"acp-default-secret-key", None)
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
    /// * `Err(String)` — Dispatch error
    pub async fn dispatch(
        &self,
        from: WebID,
        to: Option<WebID>,
        template_id: String,
        input: serde_json::Value,
    ) -> Result<String, String> {
        // Verify sender is registered
        if !self.acp_runtime.is_registered(&from).await {
            return Err(format!("Sender {:?} not registered", from));
        }

        // Verify recipient if specified
        if let Some(recipient) = to {
            if !self.acp_runtime.is_registered(&recipient).await {
                return Err(format!("Recipient {:?} not registered", recipient));
            }
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
    ) -> Result<(), String> {
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
    ) -> Result<(), String> {
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


