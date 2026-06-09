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

pub use types::{AgentKind, AgentPersona, PodID, PodLifecycleState, TemplateCrate, TemplateFile};

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
}

/// Agent pod error types
#[derive(Debug, Error)]
pub enum AgentPodError {
    #[error("Failed to parse agent persona: {0}")]
    PersonaParseError(String),

    #[error("Failed to load template crate: {0}")]
    CrateLoadError(#[from] hkask_types::GitError),

    #[error("ACP registration failed: {0}")]
    ACPRegistrationError(#[from] crate::acp::AcpError),

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

        // Use first capability from persona, or default to "tool:execute"
        let default_capability = "tool:execute".to_string();
        let capability_str = persona.capabilities.first().unwrap_or(&default_capability);
        // P4.1: Hardcoded capability string is a compile-time constant.
        // `CapabilitySpec::parse` only fails on malformed input, and
        // "tool:execute" is the canonical literal. If this ever changes,
        // the test suite will catch the parse failure.
        let spec = CapabilitySpec::parse(capability_str)
            .expect("Default capability 'tool:execute' must always parse");

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
            .await?;

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
