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
    KeyDerivation(#[from] hkask_keystore::KeystoreError),

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

/// Variant payloads as named structs so visitors and external callers can
/// match on a type name rather than reach into enum-tuple field positions.
///
/// Each struct mirrors the corresponding `A2AMessage` arm. Constructed only by
/// `A2AMessage::visit`; not `pub` outside the module.
pub struct TemplateDispatch<'a> {
    pub from: &'a WebID,
    pub to: Option<WebID>,
    pub template_id: &'a str,
    pub input: &'a serde_json::Value,
    pub correlation_id: &'a str,
}

pub struct TemplateResponse<'a> {
    pub correlation_id: &'a str,
    pub result: &'a serde_json::Value,
    pub error: Option<&'a str>,
}

pub struct MemoryArtifact<'a> {
    pub producer: &'a WebID,
    pub artifact_type: &'a str,
    pub artifact_id: &'a str,
    pub visibility: &'a str,
}

/// Visitor for `A2AMessage`.
///
/// Replaces match-on-variant for "ask the message a question" — adding a new
/// variant means adding one `on_*` method (default no-op) here and one arm to
/// `A2AMessage::visit`; the existing extraction sites do not change.
///
/// Every method is defaulted to a no-op so concrete visitors only override
/// the variants they care about (Visitor pattern, Gamma et al.).
pub trait A2AMessageVisitor {
    fn on_template_dispatch(&mut self, _msg: TemplateDispatch<'_>) {}
    fn on_template_response(&mut self, _msg: TemplateResponse<'_>) {}
    fn on_memory_artifact(&mut self, _msg: MemoryArtifact<'_>) {}
}

// Compile-time guard: the visitor trait must remain object-safe because
// `A2AMessage::visit` takes `&mut dyn A2AMessageVisitor`. Wrapped in
// `#[allow(dead_code)]` because the body never runs at runtime.
#[allow(dead_code)]
const _: fn() = || {
    fn assert_obj_safe(_: &dyn A2AMessageVisitor) {}
};

/// Internal visitor: extracts the four routing fields `send_message` needs
/// (`from`, `to`, `correlation_id` with the artifact: prefix, message_type)
/// in a single dispatch. Replaces four separate match-on-variant blocks.
#[allow(dead_code)] // used only in the test module — pub(super) trips dead_code analysis
pub(super) struct RouteFields<'a> {
    pub(super) from: Option<WebID>,
    pub(super) to: Option<WebID>,
    pub(super) correlation_id: String,
    pub(super) message_type: &'static str,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl A2AMessageVisitor for RouteFields<'_> {
    fn on_template_dispatch(&mut self, msg: TemplateDispatch<'_>) {
        self.from = Some(*msg.from);
        self.to = msg.to;
        self.correlation_id = msg.correlation_id.to_string();
        self.message_type = "template_dispatch";
    }
    fn on_template_response(&mut self, msg: TemplateResponse<'_>) {
        self.correlation_id = msg.correlation_id.to_string();
        self.message_type = "template_response";
    }
    fn on_memory_artifact(&mut self, msg: MemoryArtifact<'_>) {
        self.from = Some(*msg.producer);
        self.correlation_id = format!("artifact:{}", msg.artifact_id);
        self.message_type = "memory_artifact";
    }
}

impl A2AMessage {
    /// Dispatch a visitor over the variant. Single match site in the codebase.
    pub fn visit(&self, visitor: &mut dyn A2AMessageVisitor) {
        match self {
            A2AMessage::TemplateDispatch {
                from,
                to,
                template_id,
                input,
                correlation_id,
            } => visitor.on_template_dispatch(TemplateDispatch {
                from,
                to: *to,
                template_id,
                input,
                correlation_id,
            }),
            A2AMessage::TemplateResponse {
                correlation_id,
                result,
                error,
            } => visitor.on_template_response(TemplateResponse {
                correlation_id,
                result,
                error: error.as_deref(),
            }),
            A2AMessage::MemoryArtifact {
                producer,
                artifact_type,
                artifact_id,
                visibility,
            } => visitor.on_memory_artifact(MemoryArtifact {
                producer,
                artifact_type,
                artifact_id,
                visibility,
            }),
        }
    }

    /// Get the sender's WebID regardless of message type.
    ///
    /// Returns `Some` for `TemplateDispatch` (from) and `MemoryArtifact` (producer),
    /// `None` for `TemplateResponse` (no sender).
    pub fn from_webid(&self) -> Option<&WebID> {
        match self {
            A2AMessage::TemplateDispatch { from, .. } => Some(from),
            A2AMessage::TemplateResponse { .. } => None,
            A2AMessage::MemoryArtifact { producer, .. } => Some(producer),
        }
    }

    /// Get the correlation/artifact ID for this message.
    ///
    /// All variants carry an identifier that serves as a correlation key:
    /// `TemplateDispatch` and `TemplateResponse` use `correlation_id`,
    /// `MemoryArtifact` uses `artifact_id`.
    pub fn correlation_id(&self) -> &str {
        match self {
            A2AMessage::TemplateDispatch { correlation_id, .. } => correlation_id,
            A2AMessage::TemplateResponse { correlation_id, .. } => correlation_id,
            A2AMessage::MemoryArtifact { artifact_id, .. } => artifact_id,
        }
    }

    /// Get a human-readable message type name.
    pub fn message_type(&self) -> &'static str {
        match self {
            A2AMessage::TemplateDispatch { .. } => "template_dispatch",
            A2AMessage::TemplateResponse { .. } => "template_response",
            A2AMessage::MemoryArtifact { .. } => "memory_artifact",
        }
    }
}

// DDMVSS P8 invariant: the visitor dispatch must visit exactly one variant,
// and `RouteFields` must produce the routing fields `send_message` consumes.
// The first test pins the bijection between enum variants and visitor
// methods; the rest pin the per-variant routing invariants.

/// Consolidated mutable ACP state behind a single lock (P2.1).
///
/// Replaces 5 independent `Arc<RwLock<....>` fields with one
/// lock, eliminating dead-lock potential from multi-lock acquisitions
/// and guaranteeing consistent snapshots across read-modify-write ops.
#[derive(Default)]
struct AcpState {
    agents: HashMap<WebID, AcpAgent>,
    pending_messages: HashMap<String, A2AMessage>,
    capability_tokens: HashMap<WebID, Vec<DelegationToken>>,
    agent_secrets: HashMap<WebID, AgentSecret>,
    revoked_tokens: std::collections::HashSet<String>,
}

pub struct AcpRuntime {
    state: Arc<RwLock<AcpState>>,
    secret: Arc<Zeroizing<Vec<u8>>>,
    audit_log: Arc<AuditLog>,
    root_authority: Arc<RootAuthority>,
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
            state: Arc::new(RwLock::new(AcpState::default())),
            secret: secret_arc,
            audit_log: Arc::new(AuditLog::new()),
            root_authority,
        }
    }

    /// Keys are cryptographically independent — compromising one doesn't compromise others.
    pub async fn derive_agent_secret(&self, agent_webid: &WebID) -> AgentSecret {
        // Check cache first
        {
            let state = self.state.read().await;
            if let Some(key) = state.agent_secrets.get(agent_webid) {
                return Arc::clone(key);
            }
        }

        // Derive using HKDF-SHA256 with agent WebID as domain separator
        let context = format!("hkask:acp-agent:{}", agent_webid);
        let derived = hkask_keystore::derive_sub_key(self.secret.as_ref(), &context);
        let arc_key = Arc::new(derived);

        // Cache the derived key
        {
            let mut state = self.state.write().await;
            state
                .agent_secrets
                .insert(*agent_webid, Arc::clone(&arc_key));
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

        // Store agent and capabilities under single lock
        {
            let mut state = self.state.write().await;
            if state.agents.contains_key(&webid) {
                return Err(AcpError::AgentAlreadyRegistered(webid));
            }
            state.agents.insert(webid, agent);
            state.capability_tokens.insert(webid, tokens_vec);
        }

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
        let mut state = self.state.write().await;

        if state.agents.remove(webid).is_none() {
            return Err(AcpError::AgentNotFound(*webid));
        }

        // Remove capability tokens and per-agent derived key
        state.capability_tokens.remove(webid);
        state.agent_secrets.remove(webid);

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
        let mut state = self.state.write().await;

        let count = agents.len();

        for agent in agents {
            state.agents.insert(agent.webid, agent);
        }

        for (webid, token_list) in tokens {
            state.capability_tokens.insert(webid, token_list);
        }

        info!(
            target: "hkask.acp",
            agent_count = count,
            "Agent state restored from storage"
        );

        Ok(count)
    }

    pub(crate) async fn is_registered(&self, webid: &WebID) -> bool {
        let state = self.state.read().await;
        state.agents.contains_key(webid)
    }

    pub(crate) async fn send_message(&self, message: A2AMessage) -> Result<String, AcpError> {
        let from = message.from_webid().copied();
        let to = match &message {
            A2AMessage::TemplateDispatch { to, .. } => *to,
            _ => None,
        };
        let correlation_id = match &message {
            A2AMessage::MemoryArtifact { .. } => format!("artifact:{}", message.correlation_id()),
            _ => message.correlation_id().to_string(),
        };
        let message_type = message.message_type().to_string();

        let mut state = self.state.write().await;
        state
            .pending_messages
            .insert(correlation_id.clone(), message);

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
        let mut state = self.state.write().await;
        state.revoked_tokens.insert(token_id.to_string());
    }

    /// Get all delegation tokens for agent
    pub(crate) async fn get_capabilities(&self, webid: &WebID) -> Vec<DelegationToken> {
        let state = self.state.read().await;
        state
            .capability_tokens
            .get(webid)
            .cloned()
            .unwrap_or_default()
    }

    /// List all registered agents
    pub async fn list_agents(&self) -> Vec<AcpAgent> {
        let state = self.state.read().await;
        state.agents.values().cloned().collect()
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
    /// Construct a default ACP runtime using the resolved ACP secret.
    ///
    /// P4.1: The `Default` trait cannot return `Result`, so this is a
    /// documented panic if the secret is unavailable. The panic message
    /// is the actionable onboarding instruction ("run `kask chat` to
    /// complete onboarding, or set HKASK_MASTER_KEY or HKASK_ACP_SECRET").
    /// Callers that need graceful failure should call
    /// `hkask_keystore::resolve_acp_secret()` directly and handle the
    /// `Result` instead of using `Default::default()`.
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
        self.state
            .read()
            .await
            .agents
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
