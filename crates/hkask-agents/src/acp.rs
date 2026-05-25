//! ACP Runtime — Simplified
//!
//! Generic ACP processor reading configuration from acp-runtime.yaml.
//! Rust is the loom. YAML is the thread.
//! ℏKask v0.21.2

use crate::ports::{AuditLogStoragePort, AuditStorageEntry};
use hkask_cns::rate_limit::{RateLimitConfig, RateLimiter};
use hkask_cns::spans::SpanEmitter;
use hkask_types::WebID;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::info;

<<<<<<< HEAD
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

    let action_str = parts.last().unwrap();
    let action = CapabilityAction::parse_str(action_str).unwrap_or(CapabilityAction::Execute);

    Ok((resource, action))
=======
/// ACP configuration (loaded from YAML)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpConfig {
    pub root_authority: RootAuthorityConfig,
    #[serde(default)]
    pub agent_registration: AgentRegistrationConfig,
    #[serde(default)]
    pub messaging: MessagingConfig,
    #[serde(default)]
    pub rate_limiting: RateLimitingConfig,
    #[serde(default)]
    pub capability_verification: CapabilityVerificationConfig,
    #[serde(default)]
    pub audit: AuditConfig,
    #[serde(default)]
    pub cns: CnsIntegrationConfig,
    #[serde(default)]
    pub security: SecurityConfig,
>>>>>>> origin/main
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootAuthorityConfig {
    pub webid: String,
    pub secret: SecretConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretConfig {
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub env_var: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentRegistrationConfig {
    #[serde(default)]
    pub requirements: RegistrationRequirements,
    #[serde(default)]
    pub validation: RegistrationValidation,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegistrationRequirements {
    #[serde(default)]
    pub require_webid: bool,
    #[serde(default)]
    pub require_capabilities: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegistrationValidation {
    #[serde(default)]
    pub reject_wildcard_capabilities: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessagingConfig {
    #[serde(default)]
    pub correlation: CorrelationConfig,
    #[serde(default)]
    pub routing: RoutingConfig,
    #[serde(default)]
    pub delivery: DeliveryConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CorrelationConfig {
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoutingConfig {
    #[serde(default)]
    pub strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeliveryConfig {
    #[serde(default)]
    pub guarantee: String,
    #[serde(default)]
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RateLimitingConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub defaults: RateLimitDefaults,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RateLimitDefaults {
    #[serde(default = "default_messages_per_minute")]
    pub messages_per_minute: u64,
    #[serde(default = "default_burst_size")]
    pub burst_size: u64,
}

fn default_messages_per_minute() -> u64 {
    60
}

fn default_burst_size() -> u64 {
    10
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CapabilityVerificationConfig {
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub token_validation: TokenValidationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenValidationConfig {
    #[serde(default)]
    pub check_signature: bool,
    #[serde(default)]
    pub check_expiry: bool,
    #[serde(default = "default_max_depth")]
    pub max_chain_depth: u8,
}

fn default_max_depth() -> u8 {
    7
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuditConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub log: AuditLogging,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuditLogging {
    #[serde(default)]
    pub message_send: bool,
    #[serde(default)]
    pub message_receive: bool,
    #[serde(default)]
    pub capability_check: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CnsIntegrationConfig {
    #[serde(default)]
    pub spans: SpanEmissionConfig,
    #[serde(default)]
    pub namespace: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpanEmissionConfig {
    #[serde(default)]
    pub emit_on_message_send: bool,
    #[serde(default)]
    pub emit_on_capability_denied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityConfig {
    #[serde(default)]
    pub injection_prevention: InjectionPrevention,
    #[serde(default)]
    pub dos_prevention: DosPrevention,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InjectionPrevention {
    #[serde(default)]
    pub sanitize_message_content: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DosPrevention {
    #[serde(default)]
    pub max_message_size_bytes: usize,
}

/// ACP error types
#[derive(Debug, Error)]
pub enum AcpError {
    #[error("Agent {0:?} already registered")]
    AgentAlreadyRegistered(WebID),
    #[error("Agent {0:?} not found")]
    AgentNotFound(WebID),
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    #[error("Capability denied: {0}")]
    CapabilityDenied(String),
    #[error("Invalid capability: wildcards not allowed")]
    WildcardCapabilityNotAllowed,
<<<<<<< HEAD

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

    #[error("{0}")]
    LegacyError(String),
}

impl From<String> for AcpError {
    fn from(s: String) -> Self {
        AcpError::LegacyError(s)
    }
=======
    #[error("Secret not configured")]
    SecretNotConfigured,
>>>>>>> origin/main
}

/// A2A message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AMessage {
    pub from: WebID,
    pub to: WebID,
    pub content: serde_json::Value,
    pub correlation_id: Option<String>,
}

/// ACP Agent
#[derive(Debug, Clone)]
<<<<<<< HEAD
pub struct RootAuthority {
    /// Root authority WebID (system identity)
    root_webid: WebID,
    /// Root secret for HMAC signing (Arc to avoid copying on Clone)
    root_secret: Arc<Zeroizing<Vec<u8>>>,
    /// Next token ID counter
    token_counter: Arc<RwLock<u64>>,
}

impl RootAuthority {
    /// Create new root authority
    pub fn new(root_webid: WebID, root_secret: &[u8]) -> Self {
        Self {
            root_webid,
            root_secret: Arc::new(Zeroizing::new(root_secret.to_vec())),
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
            self.root_secret.as_ref(),
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
=======
>>>>>>> origin/main
pub struct AcpAgent {
    pub webid: WebID,
    pub capabilities: Vec<String>,
}

/// Template dispatch handler trait
pub trait TemplateDispatchHandler: Send + Sync {
    fn dispatch(&self, message: A2AMessage) -> Result<serde_json::Value, AcpError>;
}

/// ACP Runtime
pub struct AcpRuntime {
<<<<<<< HEAD
    /// Registered agents
    agents: Arc<RwLock<HashMap<WebID, AcpAgent>>>,
    /// Pending messages (correlation_id -> message)
    pending_messages: Arc<RwLock<HashMap<String, A2AMessage>>>,
    /// Capability tokens indexed by holder WebID
    capability_tokens: Arc<RwLock<HashMap<WebID, Vec<CapabilityToken>>>>,
    /// Secret for HMAC signing (Arc<Zeroizing> to avoid copying on Clone)
    secret: Arc<Zeroizing<Vec<u8>>>,
    /// Rate limiter for DoS prevention
    _rate_limiter: RateLimiter,
    /// Audit log for A2A message tracking
    audit_log: Arc<AuditLog>,
    /// Root authority for OCAP capability delegation
    root_authority: Arc<RootAuthority>,
    /// Revoked capability token IDs
    revoked_tokens: Arc<RwLock<std::collections::HashSet<String>>>,
    /// CNS emitter for observability (optional)
    cns_emitter: std::sync::RwLock<Option<Arc<dyn hkask_cns::CnsEmit + Send + Sync>>>,
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

        let secret = Arc::new(Zeroizing::new(secret_str.into_bytes()));
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
            cns_emitter: std::sync::RwLock::new(None),
        })
    }

    /// Create new ACP runtime with explicit secret
    ///
    /// # Arguments
    ///
    /// * `secret` - HMAC secret key (will be zeroized on drop)
    /// * `rate_limit_config` - Rate limit configuration (default: 100 msg/min)
    pub fn new(secret: &[u8], rate_limit_config: Option<RateLimitConfig>) -> Self {
        // Derive root WebID deterministically from a fixed "root" persona
        let root_persona = b"hkask-root-authority";
        let root_webid = WebID::from_persona(root_persona);
        let root_authority = Arc::new(RootAuthority::new(root_webid, secret));
        let secret_arc = Arc::new(Zeroizing::new(secret.to_vec()));

=======
    config: AcpConfig,
    emitter: SpanEmitter,
    agents: Arc<RwLock<HashMap<WebID, AgentInfo>>>,
    rate_limiter: RateLimiter,
}

#[derive(Debug, Clone)]
struct AgentInfo {
    capabilities: Vec<String>,
}

impl AcpRuntime {
    pub fn new(config: AcpConfig) -> Self {
        let observer = WebID::new();
        let rate_config = RateLimitConfig {
            max_tokens: config.rate_limiting.defaults.burst_size as u32,
            refill_interval: std::time::Duration::from_millis(
                60000 / config.rate_limiting.defaults.messages_per_minute,
            ),
        };
>>>>>>> origin/main
        Self {
            config,
            emitter: SpanEmitter::new(observer),
            agents: Arc::new(RwLock::new(HashMap::new())),
<<<<<<< HEAD
            pending_messages: Arc::new(RwLock::new(HashMap::new())),
            capability_tokens: Arc::new(RwLock::new(HashMap::new())),
            secret: secret_arc,
            _rate_limiter: RateLimiter::new(rate_limit_config.unwrap_or_else(|| RateLimitConfig {
                max_tokens: 100,
                refill_interval: std::time::Duration::from_millis(600),
            })),
            audit_log: Arc::new(AuditLog::new()),
            root_authority,
            revoked_tokens: Arc::new(RwLock::new(std::collections::HashSet::new())),
            cns_emitter: std::sync::RwLock::new(None),
        }
    }

    /// Set CNS emitter for observability
    pub fn with_cns_emitter(self, emitter: Arc<dyn hkask_cns::CnsEmit + Send + Sync>) -> Self {
        *self.cns_emitter.write().unwrap() = Some(emitter);
        self
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
=======
            rate_limiter: RateLimiter::new(rate_config),
        }
    }

    /// Register agent
>>>>>>> origin/main
    pub async fn register_agent(
        &self,
        webid: WebID,
        capabilities: Vec<String>,
<<<<<<< HEAD
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

        // Emit CNS span for capability minting
        if let Some(ref cns) = *self.cns_emitter.read().unwrap() {
            cns.emit_event(
                "cns.cap.minted",
                "minted",
                &serde_json::json!({
                    "token_id": token.id,
                    "holder": token.delegated_to.to_string(),
                    "resource": token.resource_id,
                    "action": token.action.as_str(),
                }),
                1.0,
            );
        }

        Ok(token)
    }

    /// Unregister an agent
    pub async fn unregister_agent(&self, webid: &WebID) -> Result<(), AcpError> {
        let mut agents = self.agents.write().await;

        if agents.remove(webid).is_none() {
            return Err(AcpError::AgentNotFound(*webid));
=======
    ) -> Result<(), AcpError> {
        if self
            .config
            .agent_registration
            .validation
            .reject_wildcard_capabilities
        {
            for cap in &capabilities {
                if cap.contains("*") {
                    return Err(AcpError::WildcardCapabilityNotAllowed);
                }
            }
        }

        let mut agents = self.agents.write().await;
        if agents.contains_key(&webid) {
            return Err(AcpError::AgentAlreadyRegistered(webid));
>>>>>>> origin/main
        }

        agents.insert(webid, AgentInfo { capabilities });

        info!("Agent {:?} registered", webid);
        Ok(())
    }

<<<<<<< HEAD
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
        let audit_entry = AuditLogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: current_timestamp()?,
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

    /// Verify capability token (HMAC signature + revocation check + expiry check)
    pub async fn verify_capability(&self, token: &CapabilityToken) -> bool {
        let current_time = chrono::Utc::now().timestamp();
        let valid = token.verify(self.secret.as_ref()) && !token.is_expired(current_time) && {
            let revoked = self.revoked_tokens.read().await;
            !revoked.contains(&token.id)
        };

        // Emit CNS span for capability verification
        if let Some(ref cns) = *self.cns_emitter.read().unwrap() {
            let span_name = if valid {
                "cns.cap.verified_ok"
            } else {
                "cns.cap.verified_denied"
            };
            cns.emit_event(
                span_name,
                "verified",
                &serde_json::json!({
                    "token_id": token.id,
                    "holder": token.delegated_to.to_string(),
                    "resource": token.resource_id,
                    "expired": token.is_expired(current_time),
                }),
                1.0,
            );
        }

        valid
    }

    /// Revoke a capability token by ID
    pub async fn revoke_capability(&self, token_id: &str) {
        let mut revoked = self.revoked_tokens.write().await;
        revoked.insert(token_id.to_string());

        // Emit CNS span for capability revocation
        if let Some(ref cns) = *self.cns_emitter.read().unwrap() {
            cns.emit_event(
                "cns.cap.revoked",
                "revoked",
                &serde_json::json!({
                    "token_id": token_id,
                }),
                1.0,
            );
        }
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

        // Emit CNS span for capability attenuation
        if let Some(ref cns) = *self.cns_emitter.read().unwrap() {
            cns.emit_event(
                "cns.cap.attenuated",
                "attenuated",
                &serde_json::json!({
                    "parent_id": parent_token.id,
                    "child_id": child.id,
                    "attenuation_level": child.attenuation_level,
                    "holder": child.delegated_to.to_string(),
                }),
                1.0,
            );
        }

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
    store: Option<Arc<dyn AuditLogStoragePort>>,
}

impl AuditLog {
    /// Create new audit log with default max entries
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            max_entries: 10000,
            store: None,
        }
    }

    /// Create audit log with custom max entries
    pub fn with_max_entries(max_entries: usize) -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            max_entries,
            store: None,
        }
    }

    pub fn with_store(store: Arc<dyn AuditLogStoragePort>) -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            max_entries: 10000,
            store: Some(store),
        }
    }

    pub fn with_max_entries_and_store(
        max_entries: usize,
        store: Arc<dyn AuditLogStoragePort>,
    ) -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            max_entries,
            store: Some(store),
        }
    }

    pub async fn log(&self, entry: AuditLogEntry) {
        if let Some(ref store) = self.store {
            let storage_entry = AuditStorageEntry {
                id: entry.id.clone(),
                timestamp: 0,
                actor_webid: entry.from.to_string(),
                action: entry.event_type.clone(),
                resource: entry.message_type.clone(),
                outcome: "success".to_string(),
                details: Some(serde_json::json!({
                    "correlation_id": entry.correlation_id,
                    "to": entry.to.map(|t| t.to_string()),
                    "metadata": entry.metadata,
                })),
                ip_address: None,
            };
            if let Err(e) = store.insert(&storage_entry) {
                tracing::error!(
                    target: "cns.audit.write_failed",
                    error = %e,
                    event_type = %entry.event_type,
                    from = %entry.from,
                    "Audit log storage write failed"
                );
            }
        }

        let mut entries = self.entries.write().await;
        entries.push(entry);

        if entries.len() > self.max_entries {
            let drain_count = entries.len() - self.max_entries;
            entries.drain(0..drain_count);
        }
    }

    pub async fn get_recent(&self, count: usize) -> Vec<AuditLogEntry> {
        if let Some(ref store) = self.store
            && let Ok(storage_entries) = store.query_recent(count)
        {
            return storage_entries
                .into_iter()
                .filter_map(audit_entry_from_port)
                .collect();
        }
        let entries = self.entries.read().await;
        entries.iter().rev().take(count).cloned().collect()
    }

    pub async fn get_by_webid(&self, webid: &WebID, count: usize) -> Vec<AuditLogEntry> {
        if let Some(ref store) = self.store
            && let Ok(storage_entries) = store.query_by_actor(&webid.to_string(), count)
        {
            return storage_entries
                .into_iter()
                .filter_map(audit_entry_from_port)
                .collect();
        }
        let entries = self.entries.read().await;
        entries
            .iter()
            .filter(|e| e.from == *webid || e.to == Some(*webid))
            .rev()
            .take(count)
            .cloned()
            .collect()
    }

    pub async fn get_by_correlation(&self, correlation_id: &str) -> Vec<AuditLogEntry> {
        let entries = self.entries.read().await;
        entries
            .iter()
            .filter(|e| e.correlation_id == correlation_id)
            .cloned()
            .collect()
    }
}

fn audit_entry_from_port(e: AuditStorageEntry) -> Option<AuditLogEntry> {
    let details = e.details.as_ref()?;
    let correlation_id = details.get("correlation_id")?.as_str()?.to_string();
    let to = details
        .get("to")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(WebID::from_string);
    let metadata = details
        .get("metadata")
        .cloned()
        .unwrap_or(serde_json::json!({}));
    Some(AuditLogEntry {
        id: e.id,
        timestamp: e.timestamp,
        from: WebID::from_string(&e.actor_webid),
        to,
        message_type: e.resource,
        correlation_id,
        event_type: e.action,
        metadata,
    })
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
        if let Some(ref store) = self.store {
            let storage_entry = AuditStorageEntry {
                id: entry.id.clone(),
                timestamp: 0,
                actor_webid: entry.from.to_string(),
                action: entry.event_type.clone(),
                resource: entry.message_type.clone(),
                outcome: "success".to_string(),
                details: Some(serde_json::json!({
                    "correlation_id": entry.correlation_id,
                    "to": entry.to.map(|t| t.to_string()),
                    "metadata": entry.metadata,
                })),
                ip_address: None,
            };
            if let Err(e) = store.insert(&storage_entry) {
                tracing::error!(
                    target: "cns.audit.write_failed",
                    error = %e,
                    event_type = %entry.event_type,
                    from = %entry.from,
                    "Audit log storage write failed (port impl)"
                );
            }
        }

        let mut entries = self.entries.write().await;
        entries.push(entry);

        if entries.len() > self.max_entries {
            let drain_count = entries.len() - self.max_entries;
            entries.drain(0..drain_count);
        }
    }

    async fn get_recent(&self, count: usize) -> Vec<AuditLogEntry> {
        if let Some(ref store) = self.store
            && let Ok(entries) = store.query_recent(count)
        {
            return entries
                .into_iter()
                .filter_map(audit_entry_from_port)
                .collect();
        }

        let entries = self.entries.read().await;
        entries.iter().rev().take(count).cloned().collect()
    }

    async fn get_by_webid(&self, webid: &WebID, count: usize) -> Vec<AuditLogEntry> {
        if let Some(ref store) = self.store
            && let Ok(entries) = store.query_by_actor(&webid.to_string(), count)
        {
            return entries
                .into_iter()
                .filter_map(audit_entry_from_port)
                .collect();
        }

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
        let keychain = hkask_keystore::Keychain::new("hkask");
        let secret = hkask_keystore::resolve(&hkask_types::SecretRef::env("HKASK_ACP_SECRET_KEY"))
            .or_else(|_| keychain.retrieve_by_key("acp-secret").map(|s| zeroize::Zeroizing::new(s.into_bytes())))
            .unwrap_or_else(|_| {
                let generated: String = (0..32)
                    .map(|_| rand::random::<u8>())
                    .map(|b| format!("{:02x}", b))
                    .collect();
                match keychain.store_by_key("acp-secret", &generated) {
                    Ok(()) => info!(target: "hkask.acp", "Generated and stored new ACP secret in OS keychain"),
                    Err(_) => tracing::warn!("Failed to store ACP secret in OS keychain; secret will not persist across restarts"),
                }
                zeroize::Zeroizing::new(generated.into_bytes())
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

    async fn revoke_capability(&self, token_id: &str, _holder: &WebID) -> Result<(), AcpError> {
        AcpRuntime::revoke_capability(self, token_id).await;
        Ok(())
    }

    async fn get_capabilities(&self, webid: &WebID) -> Vec<CapabilityToken> {
        AcpRuntime::get_capabilities(self, webid).await
    }

    fn set_cns_emitter(&self, emitter: Arc<dyn hkask_cns::CnsEmit + Send + Sync>) {
        let mut cns = self.cns_emitter.write().unwrap();
        *cns = Some(emitter);
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
=======
    /// Send A2A message
    pub async fn send_message(
        &self,
        from: WebID,
        to: WebID,
        content: serde_json::Value,
    ) -> Result<serde_json::Value, AcpError> {
        // Rate limit check
        if self.config.rate_limiting.enabled && !self.rate_limiter.check(&from) {
            self.emitter.emit_tool(
                "cns.agent.acp.rate_limit",
                serde_json::json!({"from": from.to_string(), "to": to.to_string()}),
            );
            return Err(AcpError::RateLimitExceeded);
        }

        // Capability verification
        if self.config.capability_verification.required {
            self.verify_capability(&from, "tool:execute").await?;
>>>>>>> origin/main
        }

        self.emitter.emit_tool(
            "cns.agent.acp.message.send",
            serde_json::json!({
                "from": from.to_string(),
                "to": to.to_string(),
            }),
        );

        Ok(content)
    }

<<<<<<< HEAD
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
=======
    async fn verify_capability(&self, webid: &WebID, required: &str) -> Result<(), AcpError> {
        let agents = self.agents.read().await;
        let agent = agents.get(webid).ok_or(AcpError::AgentNotFound(*webid))?;
>>>>>>> origin/main

        if !agent.capabilities.iter().any(|c| c == required) {
            return Err(AcpError::CapabilityDenied(format!(
                "Agent lacks {}",
                required
            )));
        }
        Ok(())
    }
}

/// Load ACP config from YAML
pub fn load_acp_config(yaml_path: &str) -> Result<AcpConfig, AcpError> {
    let content = std::fs::read_to_string(yaml_path).map_err(|_| AcpError::SecretNotConfigured)?;

    serde_yaml::from_str(&content).map_err(|_| AcpError::SecretNotConfigured)
}
