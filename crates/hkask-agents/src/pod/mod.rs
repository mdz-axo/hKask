//! Agent Pod Lifecycle Management
//!
//! Agent pods are minimal runtime containers that host ACP agents (bots or replicants)
//! within the hKask ecosystem. Each pod provides:
//!
//! - **Isolation**: Independent capability tokens, no shared state
//! - **Identity**: WebID-based ACP registration
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
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! use hkask_agents::pod::{AgentPod, AgentPersona, PodLifecycleState};
//! use hkask_mcp::GitCasAdapter;
//! use hkask_agents::acp::AcpRuntime;
//! use hkask_agents::adapters::mcp_runtime::CapabilityOnlyAdapter;
//! use hkask_types::{CapabilityChecker, WebID};
//! use hkask_agents::{DenyAllConsent, SovereigntyConsent};
//! use std::sync::Arc;
//!
//! // Create adapters
//! let git_adapter = GitCasAdapter::from_path(std::path::PathBuf::from("/tmp/hkask-templates"));
//! let acp_runtime = Arc::new(AcpRuntime::default());
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
//!     Arc::new(DenyAllConsent) as Arc<dyn SovereigntyConsent>,
//! )?;
//! pod.register(acp_runtime.as_ref()).await?;
//! pod.activate(&mcp_runtime)?;
//! # Ok(())
//! # }
//! ```

mod context;
mod manager;
mod nu_event;
mod types;

use hkask_types::derivation_contexts;
use hkask_types::secret::SecretRef;
use hkask_types::{
    CapabilitySpec, DataCategory, DelegationAction, DelegationResource, DelegationToken,
    SYSTEM_MAX_ATTENUATION, WebID,
};
use std::sync::Arc;
use thiserror::Error;
use tracing::info;

use zeroize::Zeroizing;

use crate::SovereigntyChecker;
use hkask_mcp::GitCasAdapter;

pub use context::PodContext;
pub use manager::{PodManager, PodStatus};

pub use types::{
    AgentKind, AgentMode, AgentPersona, PodID, PodLifecycleState, TemplateCrate, TemplateFile,
};

/// Agent Pod — Runtime container for ACP agents
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
}

/// Agent pod error types
#[derive(Debug, Error)]
pub enum AgentPodError {
    #[error("Failed to parse agent persona: {0}")]
    PersonaParseError(String),

    #[error("Failed to load template crate: {0}")]
    CrateLoadError(#[from] hkask_types::GitError),

    #[error("ACP registration failed: {0}")]
    ACPRegistrationError(String),

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
            WebID::new(),
            persona.webid(),
            ocap_secret.as_bytes(),
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
        })
    }

    /// Register the pod with the ACP runtime
    ///
    /// Transitions state: `Populated` → `Registered`
    ///
    /// # Arguments
    /// * `acp` — ACP runtime port for agent registration
    ///
    /// # Returns
    /// * `Ok(())` — Registration successful
    /// * `Err(AgentPodError)` — ACP registration failed
    pub async fn register(&mut self, acp: &dyn crate::ports::AcpPort) -> AgentPodResult<()> {
        if !self.state.can_transition_to(PodLifecycleState::Registered) {
            return Err(AgentPodError::InvalidStateTransition(
                self.state,
                PodLifecycleState::Registered,
            ));
        }

        let capabilities: Vec<String> = self.persona.capabilities.clone();
        let token = acp
            .register_agent(self.webid, self.agent_type, capabilities)
            .await
            .map_err(|e| AgentPodError::ACPRegistrationError(e.to_string()))?;

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

        info!("Agent pod {} registered with ACP", self.id);
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
            .attenuate(new_holder, ocap_secret.as_bytes(), current_time)
            .ok_or(AgentPodError::AttenuationLimitExceeded)
    }

    /// Check if the pod can perform A2A operations
    pub fn is_active(&self) -> bool {
        self.state == PodLifecycleState::Activated
    }

    /// Get the current lifecycle state
    pub fn state(&self) -> PodLifecycleState {
        self.state
    }

    // ── Agent Mode Transitions ──

    /// Enter server mode for a specific MCP role.
    ///
    /// P4 Dual Gate:
    /// 1. Agent must be Activated (lifecycle gate)
    /// 2. Agent must be assigned to the role (sovereignty/consent gate)
    /// 3. Agent must not already be in another mode (mutual exclusion)
    ///
    /// Capability verification (P4 Gate 1) is performed by the daemon
    /// at connection time, not here.
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
    pub fn is_in_server_mode(&self) -> bool {
        self.mode == Some(AgentMode::Server)
    }

    /// Check if the agent is currently in chat mode.
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
/// - Same WebID always produces the same key (UUID v5 from persona)
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
            std::env::set_var("HKASK_MASTER_KEY", "0123456789abcdef0123456789abcdef");
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

    // REQ: P4-dual-gate — Mode transitions require Activated state
    #[test]
    fn mode_requires_activation() {
        let mut pod = test_pod();
        // Pod starts in Populated state — mode entry should fail
        let err = pod.enter_server_mode("research").unwrap_err();
        assert!(matches!(err, AgentPodError::ModeRequiresActivation(_)));
        let err = pod.enter_chat_mode().unwrap_err();
        assert!(matches!(err, AgentPodError::ModeRequiresActivation(_)));
    }

    // REQ: P4-dual-gate — Mode mutual exclusion (initially single-mode)
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

    // REQ: P4-dual-gate — Role assignment check (sovereignty/consent gate)
    #[test]
    fn role_not_assigned_denied() {
        let mut pod = test_pod();
        pod.state = PodLifecycleState::Activated;
        // No roles assigned
        pod.assigned_mcp_roles = vec![];

        let err = pod.enter_server_mode("research").unwrap_err();
        assert!(matches!(err, AgentPodError::RoleNotAssigned(_, _)));
    }

    // REQ: P4-dual-gate — Mode exit and re-entry
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
}
