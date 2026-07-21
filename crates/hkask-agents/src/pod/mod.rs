//! Agent Pod Lifecycle Management
//!
//! Agent pods are minimal runtime containers that host A2A agents (bots or replicants)
//! within the hKask ecosystem. Each pod provides:
//!
//! - **Isolation**: Independent capability tokens, no shared state
//! - **Identity**: WebID-based A2A registration
//! - **Access**: Capability-gated MCP tool invocation
//! - **Observability**: CNS span emission for all lifecycle events
//! - **Persistence**: Memory artifact generation (episodic/semantic h_mems)
//!
//! # Lifecycle States
//!
//! ```text
//! Populated → Registered → Activated → Deactivated
//! ```rust,no_run
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
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! use hkask_agents::pod::{AgentPod, AgentPersona, PodLifecycleState};
//! use hkask_templates::TemplateCrateLoader;
//! use hkask_agents::a2a::A2ARuntime;
//! use hkask_types::WebID;
//! use hkask_capability::CapabilityChecker;
//! use hkask_agents::{DenyAllConsent, SovereigntyConsent};
//! use std::sync::Arc;
//!
//! // Create adapters
//! let loader = TemplateCrateLoader::from_path(std::path::PathBuf::from("/tmp/hkask-templates"));
//! let a2a_runtime = Arc::new(A2ARuntime::default());
//! let checker = Arc::new(CapabilityChecker::new());

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
//!     &loader,
//!     Arc::new(DenyAllConsent) as Arc<dyn SovereigntyConsent>,
//! )?;
//! pod.register(a2a_runtime.as_ref()).await?;
//! pod.activate(&mcp_runtime)?;
//! # Ok(())
//! # }
//! ```

mod active_pods;
mod context;
mod deployment;
mod types;

use crate::VoiceDesign;
use hkask_capability::{
    CapabilitySpec, DelegationAction, DelegationResource, DelegationToken, SYSTEM_MAX_ATTENUATION,
    derive_signing_key,
};
use hkask_types::DataCategory;
use hkask_types::WebID;
use std::sync::Arc;
use thiserror::Error;
use tracing::info;
use zeroize::Zeroizing;

use crate::SovereigntyChecker;
use crate::a2a::A2ARuntime;
use hkask_templates::TemplateCrateLoader;

pub use active_pods::{ActivePods, PodStatusInfo};
pub use context::{MemoryContext, PodContext};
pub use deployment::{
    PerPodCnsRuntime, PerPodStorage, PodDeployError, PodDeployment, PodFactory, PodRegistry,
};
pub use hkask_types::template::{TemplateCrate, TemplateFile};
pub use types::{AgentMode, CommunicationPosture, PodID, PodKind, PodLifecycleState};

/// Agent Pod — Runtime container for A2A agents
pub struct AgentPod {
    /// Unique pod identifier
    pub id: PodID,
    /// Agent's WebID
    pub webid: WebID,
    /// Pod name (1:1 per user; curator = "curator")
    pub name: String,
    /// Capabilities granted to this pod
    pub capabilities: Vec<String>,
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

use crate::error::MemoryError;

impl From<hkask_ports::ToolPortError> for AgentPodError {
    fn from(e: hkask_ports::ToolPortError) -> Self {
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
    ///       `AgentPersona`; `loader` is a valid `TemplateCrateLoader`; `consent`
    ///       is a valid `Arc<dyn SovereigntyConsent>`.
    /// post: Returns `Ok(AgentPod)` in `Populated` state with a derived
    ///       OCAP secret, capability token, and sovereignty checker.
    ///       Returns `Err` if template loading or key derivation fails.
    pub fn new(
        crate_name: &str,
        name: &str,
        webid: WebID,
        capabilities: Vec<String>,
        loader: &TemplateCrateLoader,
        consent: Arc<dyn crate::SovereigntyConsent>,
    ) -> AgentPodResult<Self> {
        let template_crate = loader.load_template_crate_or_synthesize(crate_name)?;

        // Sign with the system OCAP authority key (derived from the master key),
        // so the pod's CapabilityChecker — anchored to the matching public key —
        // accepts this token while rejecting forgeries (P4). The pod boundary,
        // not the token's resource field, is the OCAP perimeter (P4.1).
        let signing_key = system_ocap_signing_key()?;

        let default_capability = "tool:execute".to_string();
        let capability_str = capabilities.first().unwrap_or(&default_capability);
        let spec = CapabilitySpec::parse(capability_str).unwrap_or_else(|_| {
            tracing::warn!(
                target: "hkask.capability",
                capability = %capability_str,
                "Malformed capability — falling back to 'tool:execute'"
            );
            CapabilitySpec::parse(&default_capability)
                .expect("Default capability 'tool:execute' must always parse")
        });

        let capability_token = DelegationToken::new(
            spec.resource,
            spec.resource_id,
            spec.action,
            WebID::from_persona(b"system-pod-creator"),
            webid,
            &signing_key,
        );

        // Initialize sovereignty checker for this pod, wired to the live
        // consent port. Grants via the API or CLI are observed here.
        let sovereignty_checker = SovereigntyChecker::new(webid, consent);

        Ok(Self {
            id: PodID::new(),
            webid,
            name: name.to_string(),
            capabilities: capabilities.clone(),
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
    ///       idempotent re-registration); `a2a` is a valid `A2ARuntime`.
    /// post: On success, `self.state` is `Registered` and
    ///       `self.capability_token` is updated with the A2A-issued token.
    ///       On failure, state is unchanged.
    pub async fn register(&mut self, a2a: &A2ARuntime) -> AgentPodResult<()> {
        if !self.state.can_transition_to(PodLifecycleState::Registered) {
            return Err(AgentPodError::InvalidStateTransition(
                self.state,
                PodLifecycleState::Registered,
            ));
        }

        let capabilities: Vec<String> = self.capabilities.clone();
        let token = a2a
            .register_agent(self.webid, capabilities)
            .await
            .map_err(|e| AgentPodError::A2ARegistrationError(e.to_string()))?;

        self.capability_token = token;
        self.state = PodLifecycleState::Registered;

        tracing::debug!(
            target: "hkask.pod",
            span = "cns.agent_pod.registered",
            verb = "registered",
            pod_id = %self.id,
            webid = %self.webid,
            confidence = 1.0,
            "CNS event"
        );

        info!("Agent pod {} registered with A2A", self.id);
        Ok(())
    }

    /// Activate the pod for A2A communication.
    ///
    /// Transitions state: `Registered` → `Activated` after verifying that the
    /// pod's capability token is rooted in the configured OCAP authority.
    pub fn activate(
        &mut self,
        checker: &hkask_capability::CapabilityChecker,
    ) -> AgentPodResult<()> {
        if !self.state.can_transition_to(PodLifecycleState::Activated) {
            return Err(AgentPodError::InvalidStateTransition(
                self.state,
                PodLifecycleState::Activated,
            ));
        }

        if !checker.verify(&self.capability_token)
            || self.capability_token.delegated_to != self.webid
        {
            return Err(AgentPodError::CapabilityDenied {
                resource: hkask_capability::DelegationResource::Tool,
                action: hkask_capability::DelegationAction::Execute,
            });
        }

        self.state = PodLifecycleState::Activated;

        tracing::debug!(
            target: "hkask.pod",
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
            target: "hkask.pod",
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

        // Attenuate with the system OCAP authority key so the child token is
        // signed by the trusted root and verifies against pod checkers (P4).
        let signing_key = system_ocap_signing_key()?;

        self.capability_token
            .attenuate(new_holder, &signing_key, current_time)
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
            target: "hkask.pod",
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
            target: "hkask.pod",
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
                target: "hkask.pod",
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
            target: "hkask.pod",
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

    /// Consume the REPL/chat loop event reference from transcript bundle (S5 closure).
    /// Minimal surgical addition: no new framework, just event consumption.
    /// References: stt-tts.yaml (repl_chat_hook: repl_chat_ref), transcript.rs (repl_chat_ref field).
    pub fn consume_transcript_ref(&mut self, repl_chat_ref: Option<String>) -> String {
        match repl_chat_ref {
            Some(r) => {
                tracing::info!(target: "agent.pod", pod_id = %self.id, ref = %r, "REPL/chat loop event consumed (S5 closure)");
                format!(
                    "loop_closed: {} (agent pod chat mode: {})",
                    r,
                    self.is_in_chat_mode()
                )
            }
            None => "loop_open: no repl_chat_ref received".to_string(),
        }
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
///   Derive the system OCAP signing key from the master key.
///
/// \[NORMATIVE\] One of the two roots of trust for pod capability tokens (the
/// other is the A2A root). A pod's pre-registration token is signed with this
/// key, and pod `CapabilityChecker`s are anchored to its public key, so a token
/// from any other keypair is rejected (P4 — Clear Boundaries).
///
/// Derived ONCE per process and cached: issuance and verification both call this
/// and must agree. When the master key is unavailable the underlying secret
/// resolution falls back to a random secret; deriving twice would then produce
/// two different keys and break verification, so the first derived seed is
/// cached to guarantee a single consistent authority.
pub fn system_ocap_signing_key() -> AgentPodResult<ed25519_dalek::SigningKey> {
    use std::sync::OnceLock;
    static SYSTEM_OCAP_SEED: OnceLock<[u8; 32]> = OnceLock::new();

    if let Some(seed) = SYSTEM_OCAP_SEED.get() {
        return Ok(ed25519_dalek::SigningKey::from_bytes(seed));
    }
    let secret = hkask_keystore::keychain::get_or_create_ocap_secret().map_err(|e| {
        AgentPodError::KeyDerivation(hkask_keystore::KeystoreError::KeyDerivation(e.to_string()))
    })?;
    let derived = derive_signing_key(secret.as_slice()).to_bytes();
    let seed = *SYSTEM_OCAP_SEED.get_or_init(|| derived);
    Ok(ed25519_dalek::SigningKey::from_bytes(&seed))
}

/// Build a `CapabilityChecker` anchored to the system OCAP authority.
///
/// Callers that also accept A2A-issued tokens (e.g. registered pods) should
/// chain `.trust_root(a2a.root_public_key())`.
pub fn system_capability_checker() -> AgentPodResult<hkask_capability::CapabilityChecker> {
    Ok(hkask_capability::CapabilityChecker::with_signing_key(
        system_ocap_signing_key()?,
    ))
}

/// Resolve the canonical SQLCipher passphrase configured for this installation.
///
/// Database encryption and OCAP signing are separate concerns: pod databases use
/// `HKASK_DB_PASSPHRASE`, while capability tokens retain per-authority key derivation.
pub(crate) fn resolve_db_passphrase() -> AgentPodResult<Zeroizing<String>> {
    hkask_keystore::keychain::resolve_db_passphrase_string().map_err(|e| {
        AgentPodError::KeyDerivation(hkask_keystore::KeystoreError::KeyDerivation(e.to_string()))
    })
}

impl From<hkask_memory::MemoryPortError> for AgentPodError {
    fn from(e: hkask_memory::MemoryPortError) -> Self {
        AgentPodError::from(MemoryError::from(e))
    }
}

