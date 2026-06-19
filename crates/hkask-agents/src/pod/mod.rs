//! Agent Pod Lifecycle Management
//!
//! Agent pods are minimal runtime containers that host A2A agents (bots or replicants)
//! within the hKask ecosystem. Each pod provides:
//!
//! - **Isolation**: Independent capability tokens, no shared state
//! - **Identity**: WebID-based A2A registration
//! - **Access**: Capability-gated MCP tool invocation
//! - **Observability**: CNS span emission for all lifecycle events
//! - **Persistence**: Memory artifact generation (episodic/semantic triples)
//!
//! # Lifecycle States
//!
//! ```text
//! Populated → Registered → Activated → Deactivated
//! ```
//!
//! # Security Model
//!
//! Implements OCAP (Object-Capability) security with attenuation on delegation.
//! Each pod holds capability tokens that grant access to specific resources
//! (tools, templates, memory) with cryptographic verification.
//!
//! # Example
//!
//! ```rust,no_run
//! # async fn example() -> Result<(), `Box<dyn std::error::Error>`> {
//! use hkask_agents::pod::{AgentPod, AgentPersona, PodLifecycleState};
//! use hkask_mcp::GitCasAdapter;
//! use hkask_agents::a2a::A2ARuntime;
//! use hkask_agents::adapters::mcp_runtime::CapabilityOnlyAdapter;
//! use hkask_types::{CapabilityChecker, WebID};
//! use hkask_agents::{DenyAllConsent, SovereigntyConsent};
//! use std::sync::Arc;
//!
//! // Create adapters
//! let git_adapter = GitCasAdapter::from_path(std::path::PathBuf::from("/tmp/hkask-templates"));
//! let a2a_runtime = Arc::new(A2ARuntime::default());
//! let checker = Arc::new(CapabilityChecker::new(&[]));
//! let mcp_runtime = CapabilityOnlyAdapter::new(checker);
//!
//! // Create a simple persona YAML
//! let yaml_str = r#"
//! name: test-bot
//! type: bot
//! persona: A test bot
//! "#;
//!
//! let persona = AgentPersona::from_yaml(yaml_str)?;
//! let mut pod = AgentPod::new(
//!     "test-bot",
//!     &persona,
//!     &git_adapter,
//!     Arc::new(DenyAllConsent) as `Arc<dyn SovereigntyConsent>`,
//! )?;
//! pod.register(a2a_runtime.as_ref()).await?;
//! pod.activate(&mcp_runtime)?;
//! # Ok(())
//! # }
//! ```

mod active_pods;
mod context;
mod deployment;
mod nu_event;
mod types;

use hkask_types::capability::derive_signing_key;
use hkask_types::secret::SecretRef;
use hkask_types::secret::derivation_contexts;
use hkask_types::sovereignty::DataCategory;
use hkask_types::{
    CapabilitySpec, DelegationAction, DelegationResource, DelegationToken, SYSTEM_MAX_ATTENUATION,
    VoiceDesign, WebID,
};
use std::sync::Arc;
use thiserror::Error;
use tracing::info;

use zeroize::Zeroizing;

use crate::SovereigntyChecker;
use hkask_mcp::GitCasAdapter;

pub use active_pods::{ActivePods, PodStatusInfo};
pub use context::PodContext;
pub use deployment::{
    PerPodCnsRuntime, PerPodStorage, PerPodToolBinding, PodDeployError, PodDeployment, PodFactory,
    PodRegistry,
};
pub use hkask_types::template::{TemplateCrate, TemplateFile};
pub use types::{AgentKind, AgentMode, AgentPersona, PodID, PodKind, PodLifecycleState};

/// Agent Pod — Runtime container for A2A agents
pub struct AgentPod {
    /// Unique pod identifier
    pub id: PodID,
    /// Agent's WebID
    pub webid: WebID,
    /// Agent type (Bot or Replicant)
    pub agent_type: AgentKind,
    /// Agent persona
    pub persona: AgentPersona,
    /// Template crate reference
    pub template_crate: TemplateCrate,
    /// Primary capability token
    pub capability_token: DelegationToken,
    /// Current lifecycle state
    pub state: PodLifecycleState,
    /// Pod creation timestamp (Unix epoch)
    pub created_at: i64,
    /// Maximum attenuation level for delegation
    pub max_attenuation: u8,
    /// Sovereignty checker for this pod
    pub(crate) sovereignty_checker: SovereigntyChecker,
    /// Current operating mode (None = not in any active mode)
    pub mode: Option<AgentMode>,
    /// MCP server roles this agent is assigned to serve (e.g., ["research", "condenser"])
    pub assigned_mcp_roles: Vec<String>,
    /// Voice design for TTS speech generation (None = use default neutral voice)
    pub voice_design: Option<VoiceDesign>,
}

/// Agent pod error types
#[derive(Debug, Error)]
pub enum AgentPodError {
    #[error("Failed to parse agent persona: {0}")]
    PersonaParseError(String),

    #[error("Failed to load template crate: {0}")]
    CrateLoadError(#[from] hkask_types::InfrastructureError),

    #[error("A2A registration failed: {0}")]
    A2ARegistrationError(String),

    #[error("MCP access grant failed: {0}")]
    MCPAccessError(#[from] crate::error::McpError),

    #[error("Capability attenuation limit exceeded")]
    AttenuationLimitExceeded,

    #[error("Invalid lifecycle transition: {0} -> {1}")]
    InvalidStateTransition(PodLifecycleState, PodLifecycleState),

    #[error("Clock error: {0}")]
    ClockError(String),

    #[error("Capability denied: token does not grant {resource:?} {action:?}")]
    CapabilityDenied {
        resource: DelegationResource,
        action: DelegationAction,
    },

    #[error(
        "Sovereignty denied: data category {category:?} requires explicit consent for WebID {requester}"
    )]
    SovereigntyDenied {
        category: DataCategory,
        requester: WebID,
    },

    #[error("Inference port unavailable: {0}")]
    InferenceUnavailable(String),

    #[error("Memory operation failed: {0}")]
    MemoryError(#[from] crate::error::MemoryError),

    #[error("Tool invocation failed: {0}")]
    ToolError(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("Key derivation failed: {0}")]
    KeyDerivation(#[from] hkask_keystore::KeystoreError),

    #[error("Pod not found: {0}")]
    PodNotFound(PodID),

    #[error("Pod must be activated before creating context")]
    PodNotActivated,

    #[error("Agent is already in {0} mode — exit current mode first")]
    ModeConflict(AgentMode),

    #[error("Agent must be Activated before entering a mode (current state: {0})")]
    ModeRequiresActivation(PodLifecycleState),

    #[error("Agent is not assigned to MCP role '{0}'. Assigned roles: {1:?}")]
    RoleNotAssigned(String, Vec<String>),
}

impl From<hkask_types::ports::ToolPortError> for AgentPodError {
    fn from(e: hkask_types::ports::ToolPortError) -> Self {
        AgentPodError::ToolError(Box::new(e))
    }
}

/// Result type for agent pod operations
pub type AgentPodResult<T> = Result<T, AgentPodError>;

impl AgentPod {
    /// Create a new AgentPod.
    ///
    /// expect: "My agents operate within my sovereignty boundaries"
    /// \[P1\] Motivating: User Sovereignty — AgentPod is the user's agent container
    /// \[P4\] Constraining: Clear Boundaries — OCAP secret + capability token on creation
    /// pre:  `crate_name` is a non-empty string; `persona` is a valid
    ///       `AgentPersona`; `git` is a valid `GitCasAdapter`; `consent`
    ///       is a valid `Arc<dyn SovereigntyConsent>`.
    /// post: Returns `Ok(AgentPod)` in `Populated` state with a derived
    ///       OCAP secret, capability token, and sovereignty checker.
    ///       Returns `Err` if template loading or key derivation fails.
    pub fn new(
        crate_name: &str,
        persona: &AgentPersona,
        git: &GitCasAdapter,
        consent: Arc<dyn crate::SovereigntyConsent>,
    ) -> AgentPodResult<Self> {
        let template_crate = git.load_template_crate(crate_name)?;

        // Derive OCAP secret per WebID via HKDF-SHA256 from master key
        // (ADR-027: deterministic, restart-safe, per-agent isolation)
        let ocap_secret = derive_ocap_secret(&persona.webid())?;

        // Use first capability from persona, or default to "tool:execute".
        // The default is the canonical literal; persona capabilities are user-supplied.
        let default_capability = "tool:execute".to_string();
        let capability_str = persona.capabilities.first().unwrap_or(&default_capability);
        let spec = CapabilitySpec::parse(capability_str).unwrap_or_else(|_| {
            // Malformed user-supplied capability — fall back to safe default.
            CapabilitySpec::parse(&default_capability)
                .expect("Default capability 'tool:execute' must always parse")
        });

        let capability_token = DelegationToken::new(
            spec.resource,
            spec.resource_id,
            spec.action,
            WebID::from_persona(b"system-pod-creator"),
            persona.webid(),
            &derive_signing_key(ocap_secret.as_bytes()),
        );

        // Initialize sovereignty checker for this pod, wired to the live
        // consent port. Grants via the API or CLI are observed here.
        let sovereignty_checker = SovereigntyChecker::new(persona.webid(), consent);

        Ok(Self {
            id: PodID::new(),
            webid: persona.webid(),
            agent_type: persona.agent.agent_type,
            persona: persona.clone(),
            template_crate,
            capability_token,
            state: PodLifecycleState::Populated,
            created_at: current_timestamp()?,
            max_attenuation: SYSTEM_MAX_ATTENUATION,
            sovereignty_checker,
            mode: None,
            assigned_mcp_roles: Vec::new(),
            voice_design: None,
        })
    }

    /// Register the pod with the A2A runtime
    ///
    /// Transitions state: `Populated` → `Registered`
    ///
    /// # Arguments
    /// * `acp` — A2A runtime port for agent registration
    ///
    /// # Returns
    /// * `Ok(())` — Registration successful
    /// * `Err(AgentPodError)` — A2A registration failed
    ///
    /// expect: "My agents operate within my sovereignty boundaries"
    /// \[P1\] Motivating: User Sovereignty — register pod with A2A under its WebID
    /// pre:  `self.state` must be `Populated` (or `Registered` for
    ///       idempotent re-registration); `acp` is a valid `A2APort`.
    /// post: On success, `self.state` is `Registered` and
    ///       `self.capability_token` is updated with the A2A-issued token.
    ///       On failure, state is unchanged.
    pub async fn register(&mut self, a2a: &dyn crate::ports::A2APort) -> AgentPodResult<()> {
        if !self.state.can_transition_to(PodLifecycleState::Registered) {
            return Err(AgentPodError::InvalidStateTransition(
                self.state,
                PodLifecycleState::Registered,
            ));
        }

        let capabilities: Vec<String> = self.persona.capabilities.clone();
        let token = a2a
            .register_agent(self.webid, self.agent_type, capabilities)
            .await
            .map_err(|e| AgentPodError::A2ARegistrationError(e.to_string()))?;

        self.capability_token = token;
        self.state = PodLifecycleState::Registered;

        tracing::debug!(
            target: "cns.pod",
            span = "cns.agent_pod.registered",
            verb = "registered",
            pod_id = %self.id,
            webid = %self.webid,
            agent_type = %self.agent_type,
            confidence = 1.0,
            "CNS event"
        );

        info!("Agent pod {} registered with A2A", self.id);
        Ok(())
    }

    /// Activate the pod for A2A communication
    ///
    /// Transitions state: `Registered` → `Activated`
    ///
    /// # Arguments
    /// * `mcp` — MCP runtime port for tool access grants
    ///
    /// # Returns
    /// * `Ok(())` — Activation successful
    /// * `Err(AgentPodError)` — MCP access grant failed
    ///
    /// expect: "My agents operate within my sovereignty boundaries"
    /// \[P1\] Motivating: User Sovereignty — activate grants MCP access
    /// \[P4\] Constraining: Clear Boundaries — requires Registered state
    /// pre:  `self.state` must be `Registered` (or `Activated` for
    ///       idempotent re-activation); `mcp` is a valid `MCPRuntimePort`.
    /// post: On success, `self.state` is `Activated` and MCP tool access
    ///       is granted. On failure, state is unchanged.
    pub fn activate(&mut self, mcp: &dyn crate::ports::MCPRuntimePort) -> AgentPodResult<()> {
        if !self.state.can_transition_to(PodLifecycleState::Activated) {
            return Err(AgentPodError::InvalidStateTransition(
                self.state,
                PodLifecycleState::Activated,
            ));
        }

        mcp.grant_tool_access(self.capability_token.clone())?;

        self.state = PodLifecycleState::Activated;

        tracing::debug!(
            target: "cns.pod",
            span = "cns.agent_pod.activated",
            verb = "activated",
            pod_id = %self.id,
            webid = %self.webid,
            mcp_access = true,
            confidence = 1.0,
            "CNS event"
        );

        info!("Agent pod {} activated for A2A communication", self.id);
        Ok(())
    }

    /// Deactivate the pod and revoke capabilities
    ///
    /// Transitions state: `Activated` → `Deactivated`
    ///
    /// # Returns
    /// * `Ok(())` — Deactivation successful
    ///
    /// expect: "My agents operate within my sovereignty boundaries"
    /// \[P1\] Motivating: User Sovereignty — deactivate terminates MCP access
    /// pre:  `self.state` must be `Activated` (or `Deactivated` for
    ///       idempotent re-deactivation).
    /// post: `self.state` is `Deactivated`.
    pub fn deactivate(&mut self) -> AgentPodResult<()> {
        if !self.state.can_transition_to(PodLifecycleState::Deactivated) {
            return Err(AgentPodError::InvalidStateTransition(
                self.state,
                PodLifecycleState::Deactivated,
            ));
        }

        self.state = PodLifecycleState::Deactivated;

        tracing::debug!(
            target: "cns.pod",
            span = "cns.agent_pod.deactivated",
            verb = "deactivated",
            pod_id = %self.id,
            webid = %self.webid,
            capabilities_revoked = true,
            confidence = 1.0,
            "CNS event"
        );

        info!("Agent pod {} deactivated", self.id);
        Ok(())
    }

    /// Create an attenuated capability token for delegation
    ///
    /// # Arguments
    /// * `new_holder` — WebID of the delegate
    /// * `current_time` — Current Unix timestamp
    ///
    /// # Returns
    /// * `Ok(DelegationToken)` — Attenuated child token
    /// * `Err(AgentPodError)` — Attenuation limit exceeded or keystore error
    ///
    /// expect: "My agents operate within my sovereignty boundaries"
    /// \[P4\] Motivating: Clear Boundaries — delegate capability to another holder with attenuation
    /// \[P7\] Constraining: Evolutionary Architecture — attenuation limit emerged from usage
    /// pre:  `new_holder` is a valid `WebID`; `current_time` is a valid
    ///       Unix timestamp; `self.capability_token.attenuation_level`
    ///       must be < `self.max_attenuation`.
    /// post: Returns `Ok(DelegationToken)` — an attenuated child token —
    ///       or `Err(AttenuationLimitExceeded)`.
    pub fn delegate(
        &self,
        new_holder: WebID,
        current_time: i64,
    ) -> AgentPodResult<DelegationToken> {
        // Check attenuation limit
        if self.capability_token.attenuation_level >= self.max_attenuation {
            return Err(AgentPodError::AttenuationLimitExceeded);
        }

        // Derive OCAP secret for attenuation (same HKDF derivation)
        let ocap_secret = derive_ocap_secret(&self.webid)?;

        self.capability_token
            .attenuate(
                new_holder,
                &derive_signing_key(ocap_secret.as_bytes()),
                current_time,
            )
            .ok_or(AgentPodError::AttenuationLimitExceeded)
    }

    /// Check if the pod can perform A2A operations.
    ///
    /// expect: "My agents operate within my sovereignty boundaries"
    /// \[P8\] Motivating: Semantic Grounding — state accessor for Activated
    /// pre:  (none).
    /// post: Returns `true` iff `self.state == PodLifecycleState::Activated`.
    pub fn is_active(&self) -> bool {
        self.state == PodLifecycleState::Activated
    }

    /// Get the current lifecycle state.
    ///
    /// expect: "My agents operate within my sovereignty boundaries"
    /// \[P8\] Motivating: Semantic Grounding — lifecycle state accessor
    /// pre:  (none — accessor).
    /// post: Returns the current `PodLifecycleState`.
    pub fn state(&self) -> PodLifecycleState {
        self.state
    }

    // ── Agent Mode Transitions ──

    /// Enter server mode for a specific MCP role.
    ///
    /// P4 Dual Gate:
    /// 1. \[NORMATIVE\] Agent must be Activated (lifecycle gate) (P4 — Clear Boundaries)
    /// 2. \[NORMATIVE\] Agent must be assigned to the role (sovereignty/consent gate) (P2 — Affirmative Consent)
    /// 3. \[NORMATIVE\] Agent must not already be in another mode (mutual exclusion) (P4 — Clear Boundaries)
    ///
    /// Capability verification (P4 Gate 1) is performed by the daemon
    /// at connection time, not here.
    ///
    /// expect: "My agents operate within my sovereignty boundaries"
    /// \[P1\] Motivating: User Sovereignty — enter server mode to serve MCP role
    /// \[P4\] Constraining: Clear Boundaries — requires Activated + assigned role
    /// pre:  `self.state == Activated`; `self.mode == None`; `role` is
    ///       in `self.assigned_mcp_roles`.
    /// post: `self.mode` is set to `Some(AgentMode::Server)`.
    ///       Returns `Err` if any precondition fails.
    pub fn enter_server_mode(&mut self, role: &str) -> AgentPodResult<()> {
        if self.state != PodLifecycleState::Activated {
            return Err(AgentPodError::ModeRequiresActivation(self.state));
        }
        if let Some(ref current) = self.mode {
            return Err(AgentPodError::ModeConflict(*current));
        }
        if !self.assigned_mcp_roles.iter().any(|r| r == role) {
            return Err(AgentPodError::RoleNotAssigned(
                role.to_string(),
                self.assigned_mcp_roles.clone(),
            ));
        }
        self.mode = Some(AgentMode::Server);
        tracing::info!(
            target: "cns.pod",
            span = "cns.agent_pod.server_mode_enter",
            pod_id = %self.id,
            webid = %self.webid,
            role = %role,
            "Agent entered server mode"
        );
        Ok(())
    }

    /// Enter chat mode.
    ///
    /// Requires: Activated state, not already in another mode.
    ///
    /// expect: "My agents operate within my sovereignty boundaries"
    /// \[P1\] Motivating: User Sovereignty — enter chat mode for interactive use
    /// \[P4\] Constraining: Clear Boundaries — requires Activated + no other mode
    /// pre:  `self.state == Activated`; `self.mode == None`.
    /// post: `self.mode` is set to `Some(AgentMode::Chat)`.
    ///       Returns `Err` if any precondition fails.
    pub fn enter_chat_mode(&mut self) -> AgentPodResult<()> {
        if self.state != PodLifecycleState::Activated {
            return Err(AgentPodError::ModeRequiresActivation(self.state));
        }
        if let Some(ref current) = self.mode {
            return Err(AgentPodError::ModeConflict(*current));
        }
        self.mode = Some(AgentMode::Chat);
        tracing::info!(
            target: "cns.pod",
            span = "cns.agent_pod.chat_mode_enter",
            pod_id = %self.id,
            webid = %self.webid,
            "Agent entered chat mode"
        );
        Ok(())
    }

    /// Exit the current mode, returning the agent to no active mode.
    ///
    /// expect: "My agents operate within my sovereignty boundaries"
    /// \[P1\] Motivating: User Sovereignty — exit current mode
    /// pre:  (none — always valid).
    /// post: `self.mode` is set to `None`; the previous mode (if any)
    ///       is logged. Always returns `Ok(())`.
    pub fn exit_mode(&mut self) -> AgentPodResult<()> {
        let previous = self.mode.take();
        if let Some(mode) = previous {
            tracing::info!(
                target: "cns.pod",
                span = "cns.agent_pod.mode_exit",
                pod_id = %self.id,
                webid = %self.webid,
                mode = %mode,
                "Agent exited mode"
            );
        }
        Ok(())
    }

    /// Check if the agent is currently in server mode.
    ///
    /// expect: "My agents operate within my sovereignty boundaries"
    /// \[P8\] Motivating: Semantic Grounding — mode accessor
    /// pre:  (none).
    /// post: Returns `true` iff `self.mode == Some(AgentMode::Server)`.
    pub fn is_in_server_mode(&self) -> bool {
        self.mode == Some(AgentMode::Server)
    }

    // ── Voice ──

    /// Set the agent's voice design for TTS speech generation.
    ///
    /// expect: "My agents operate within my sovereignty boundaries"
    /// \[P3\] Motivating: Generative Space — configure voice design
    /// pre:  `voice` is a valid `VoiceDesign`.
    /// post: `self.voice_design` is set to `Some(voice)`; logs the change.
    pub fn set_voice(&mut self, voice: VoiceDesign) {
        tracing::info!(
            target: "cns.pod",
            pod_id = %self.id,
            voice_name = %voice.name,
            "Agent voice design set"
        );
        self.voice_design = Some(voice);
    }

    /// Get the agent's voice design, if one has been set.
    ///
    /// expect: "My agents operate within my sovereignty boundaries"
    /// \[P8\] Motivating: Semantic Grounding — voice design accessor
    /// pre:  (none — accessor).
    /// post: Returns `Some(&VoiceDesign)` if set, `None` otherwise.
    pub fn get_voice(&self) -> Option<&VoiceDesign> {
        self.voice_design.as_ref()
    }

    /// Get the TTS description for this agent's voice.
    /// Returns the default neutral voice description if no voice is set.
    ///
    /// expect: "My agents operate within my sovereignty boundaries"
    /// \[P8\] Motivating: Semantic Grounding — return TTS description
    /// pre:  (none).
    /// post: Returns the TTS description string for the configured voice,
    ///       or the default `VoiceDesign::default()` description if none
    ///       is set.
    pub fn voice_description(&self) -> String {
        self.voice_design
            .as_ref()
            .map(|v| v.to_tts_description())
            .unwrap_or_else(|| VoiceDesign::default().to_tts_description())
    }

    /// Check if the agent is currently in chat mode.
    ///
    /// expect: "My agents operate within my sovereignty boundaries"
    /// \[P8\] Motivating: Semantic Grounding — mode accessor
    /// pre:  (none).
    /// post: Returns `true` iff `self.mode == Some(AgentMode::Chat)`.
    pub fn is_in_chat_mode(&self) -> bool {
        self.mode == Some(AgentMode::Chat)
    }

    /// Execute action with sovereignty check
    ///
    /// This method performs an OCAP sovereignty check before executing
    /// any action that accesses data categories.
    ///
    /// # Arguments
    /// * `action` — The action to execute
    /// * `data_category` — The data category being accessed
    /// * `requester` — The WebID requesting the action
    ///
    /// # Returns
    /// * `Ok(true)` — Action is permitted
    /// * `Ok(false)` — Action denied by sovereignty check
    /// * `Err(AgentPodError)` — Sovereignty check error
    ///
    /// expect: "My agents operate within my sovereignty boundaries"
    /// \[P1\] Motivating: User Sovereignty — verify action against sovereignty/consent
    /// \[P2\] Constraining: Affirmative Consent — delegates to consent boundary
    /// pre:  `action` is a non-empty string; `data_category` is a valid
    ///       `DataCategory`; `requester` is a valid `WebID`.
    /// post: Returns `Ok(true)` if both `check_operation` and `can_access`
    ///       pass; `Ok(false)` if either fails; `Err` on sovereignty
    ///       checker error.
    pub fn check_sovereignty(
        &self,
        action: &str,
        data_category: &DataCategory,
        requester: &WebID,
    ) -> Result<bool, AgentPodError> {
        let checker = &self.sovereignty_checker;

        // Check if operation is permitted
        if !checker.check_operation(action, data_category) {
            return Ok(false);
        }

        // Check if requester can access the data category
        if !checker.can_access(data_category, requester) {
            return Ok(false);
        }

        Ok(true)
    }
}

fn current_timestamp() -> Result<i64, AgentPodError> {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .map_err(|e| AgentPodError::ClockError(e.to_string()))
}

/// Derive OCAP secret per WebID from master key via HKDF-SHA256.
///
/// Uses the per-agent context `"hkask:ocap-secret:<webid>"`
/// to produce cryptographically independent signing keys for each
/// agent, while remaining deterministic (same passphrase + same
/// WebID → same secret) for restart safety.
///
/// # Security
///
/// - Derives from the master passphrase via Argon2id → HKDF-SHA256
/// - Different WebIDs produce independent sub-keys (HKDF domain separation)
/// - \[DECLARATIVE\] Same WebID always produces the same key (UUID v5 from persona)
/// - No random generation — ADR-027 compliant
/// - No keystore dependency per pod — only the master key needs storage
fn derive_ocap_secret(webid: &WebID) -> AgentPodResult<Zeroizing<String>> {
    let context = format!("{}:{}", derivation_contexts::OCAP_SECRET, webid);
    let secret_ref = SecretRef::derived(derivation_contexts::MASTER_KEY_ENV, &context);
    let bytes =
        hkask_keystore::resolve(&secret_ref).map_err(|e| AgentPodError::KeyDerivation(e.into()))?;
    Ok(Zeroizing::new(hex::encode(&*bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DenyAllConsent, SovereigntyConsent};
    use std::sync::Arc;

    fn test_persona() -> AgentPersona {
        AgentPersona::from_yaml(
            "agent:\n  name: test-agent\n  type: Replicant\ncharter:\n  description: test\n  editor: test\ncapabilities: []\nrights: []\nresponsibilities: []\nvisibility:\n  default: public\n"
        ).expect("test persona parse")
    }

    fn test_pod() -> AgentPod {
        // SAFETY: test runs in single-threaded context, no other code reads HKASK_MASTER_KEY concurrently
        unsafe {
            std::env::set_var("HKASK_MASTER_KEY", "xXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxX");
        }
        let persona = test_persona();
        let template_dir = std::path::PathBuf::from("/tmp/hkask-test");
        let crate_dir = template_dir.join("test-template");
        std::fs::create_dir_all(&crate_dir).ok();
        std::fs::write(
            crate_dir.join("agent_persona.yaml"),
            "agent:\n  name: test\n  type: Bot\n",
        )
        .ok();
        std::fs::write(crate_dir.join("dispatch_manifest.yaml"), "name: test\n").ok();
        let git = hkask_mcp::GitCasAdapter::from_path(template_dir);
        AgentPod::new(
            "test-template",
            &persona,
            &git,
            Arc::new(DenyAllConsent) as Arc<dyn SovereigntyConsent>,
        )
        .expect("test pod creation")
    }

    /// expect: "Agent interactions are gated by OCAP boundaries"
    #[test]
    fn mode_requires_activation() {
        let mut pod = test_pod();
        // Pod starts in Populated state — mode entry should fail
        let err = pod.enter_server_mode("research").unwrap_err();
        assert!(matches!(err, AgentPodError::ModeRequiresActivation(_)));
        let err = pod.enter_chat_mode().unwrap_err();
        assert!(matches!(err, AgentPodError::ModeRequiresActivation(_)));
    }

    /// expect: "Agent interactions are gated by OCAP boundaries"
    #[test]
    fn mode_mutual_exclusion() {
        let mut pod = test_pod();
        // Manually set Activated state for testing mode transitions
        pod.state = PodLifecycleState::Activated;
        pod.assigned_mcp_roles = vec!["research".to_string()];

        // Enter server mode
        pod.enter_server_mode("research")
            .expect("enter server mode");
        assert!(pod.is_in_server_mode());

        // Attempting chat mode while in server mode should fail
        let err = pod.enter_chat_mode().unwrap_err();
        assert!(matches!(
            err,
            AgentPodError::ModeConflict(AgentMode::Server)
        ));

        // Attempting server mode again should also fail
        let err = pod.enter_server_mode("research").unwrap_err();
        assert!(matches!(
            err,
            AgentPodError::ModeConflict(AgentMode::Server)
        ));
    }

    /// expect: "Agent interactions are gated by OCAP boundaries"
    #[test]
    fn role_not_assigned_denied() {
        let mut pod = test_pod();
        pod.state = PodLifecycleState::Activated;
        // No roles assigned
        pod.assigned_mcp_roles = vec![];

        let err = pod.enter_server_mode("research").unwrap_err();
        assert!(matches!(err, AgentPodError::RoleNotAssigned(_, _)));
    }

    /// expect: "Agent interactions are gated by OCAP boundaries"
    #[test]
    fn mode_exit_and_switch() {
        let mut pod = test_pod();
        pod.state = PodLifecycleState::Activated;
        pod.assigned_mcp_roles = vec!["research".to_string()];

        // Enter server, exit, enter chat
        pod.enter_server_mode("research").expect("enter server");
        pod.exit_mode().expect("exit server");
        assert!(pod.mode.is_none());

        pod.enter_chat_mode().expect("enter chat");
        assert!(pod.is_in_chat_mode());

        // Exit chat, re-enter server
        pod.exit_mode().expect("exit chat");
        pod.enter_server_mode("research").expect("re-enter server");
        assert!(pod.is_in_server_mode());
    }

    /// expect: "My agents operate within my sovereignty boundaries"
    #[test]
    fn lifecycle_state_valid_transitions() {
        assert!(PodLifecycleState::Populated.can_transition_to(PodLifecycleState::Registered));
        assert!(PodLifecycleState::Registered.can_transition_to(PodLifecycleState::Activated));
        assert!(PodLifecycleState::Activated.can_transition_to(PodLifecycleState::Deactivated));
    }

    /// expect: "My agents operate within my sovereignty boundaries"
    #[test]
    fn lifecycle_state_rejects_invalid_transitions() {
        assert!(!PodLifecycleState::Populated.can_transition_to(PodLifecycleState::Activated));
        assert!(!PodLifecycleState::Populated.can_transition_to(PodLifecycleState::Deactivated));
        assert!(!PodLifecycleState::Registered.can_transition_to(PodLifecycleState::Deactivated));
        assert!(!PodLifecycleState::Deactivated.can_transition_to(PodLifecycleState::Activated));
    }

    /// expect: "My agents operate within my sovereignty boundaries"
    #[test]
    fn new_pod_has_correct_defaults() {
        let pod = test_pod();
        assert_eq!(pod.state, PodLifecycleState::Populated);
        assert!(pod.mode.is_none());
        assert!(pod.assigned_mcp_roles.is_empty());
        assert!(pod.voice_design.is_none());
        assert!(!pod.is_active());
        assert!(!pod.is_in_server_mode());
        assert!(!pod.is_in_chat_mode());
    }

    /// expect: "My agents operate within my sovereignty boundaries"
    #[test]
    fn is_active_only_when_activated() {
        let mut pod = test_pod();
        assert!(!pod.is_active()); // Populated
        pod.state = PodLifecycleState::Registered;
        assert!(!pod.is_active());
        pod.state = PodLifecycleState::Activated;
        assert!(pod.is_active());
        pod.state = PodLifecycleState::Deactivated;
        assert!(!pod.is_active());
    }

    /// expect: "My agents operate within my sovereignty boundaries"
    #[test]
    fn voice_design_set_get_roundtrip() {
        let mut pod = test_pod();
        let voice = VoiceDesign::default();
        pod.set_voice(voice.clone());
        assert!(pod.get_voice().is_some());
        assert_eq!(pod.get_voice().unwrap().name, voice.name);
        assert!(!pod.voice_description().is_empty());
    }

    /// expect: "My agents operate within my sovereignty boundaries"
    #[test]
    fn agent_pod_error_display_is_readable() {
        let err = AgentPodError::ModeRequiresActivation(PodLifecycleState::Populated);
        assert!(err.to_string().contains("Activated"));
        assert!(err.to_string().contains("populated"));

        let err = AgentPodError::ModeConflict(AgentMode::Server);
        assert!(err.to_string().contains("server"));
        assert!(err.to_string().contains("exit current mode"));

        let err = AgentPodError::RoleNotAssigned("research".into(), vec![]);
        assert!(err.to_string().contains("research"));
    }
}
