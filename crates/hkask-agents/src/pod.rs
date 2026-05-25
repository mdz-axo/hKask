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
//! use hkask_agents::adapters::git_cas::MockGitCas;
//! use hkask_agents::acp::AcpRuntime;
//! use hkask_agents::adapters::cns_emitter::CnsEmitterAdapter;
//! use hkask_agents::adapters::mcp_runtime::McpRuntimeAdapter;
//! use hkask_types::WebID;
//! use std::sync::Arc;
//!
//! // Create adapters
//! let git_adapter = MockGitCas::new();
//! let acp_runtime = Arc::new(AcpRuntime::default());
//! let cns_emitter = CnsEmitterAdapter::new(WebID::new());
//! let mcp_runtime = McpRuntimeAdapter::new();
//!
//! // Create a simple persona YAML
//! let yaml_str = r#"
//! name: test-bot
//! type: bot
//! persona: A test bot
//! "#;
//!
//! let persona = AgentPersona::from_yaml(yaml_str)?;
//! let mut pod = AgentPod::new("test-bot", &persona, &git_adapter)?;
//! pod.register(acp_runtime.as_ref(), &cns_emitter).await?;
//! pod.activate(&mcp_runtime, &cns_emitter)?;
//! # Ok(())
//! # }
//! ```

use hkask_cns::CnsEmit;
use hkask_keystore::keychain::Keychain;
use hkask_types::{CapabilityAction, CapabilityResource, CapabilityToken, WebID};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::info;
use zeroize::Zeroizing;

use crate::adapters::cns_emitter::CnsEmitterAdapter;
use crate::adapters::git_cas::GitCasAdapter;
use crate::adapters::mcp_runtime::McpRuntimeAdapter;
use crate::adapters::memory_storage::MemoryStorageAdapter;
<<<<<<< HEAD
use crate::ports::{GitCASPort, MCPRuntimePort, MemoryStoragePort};
use crate::security::{AgentPersonaInput, InputValidator, SecurityContext};
=======
use crate::security::{AgentPersonaInput, InputValidator, SecurityContext};
use crate::sovereignty::SovereigntyChecker;
>>>>>>> origin/main
use std::path::PathBuf;

/// Pod lifecycle state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PodLifecycleState {
    /// Pod instantiated from template crate, not yet registered
    Populated,
    /// Registered with ACP runtime, capability token minted
    Registered,
    /// Activated for A2A communication, MCP access granted
    Activated,
    /// Deactivated, capabilities revoked
    Deactivated,
}

impl std::fmt::Display for PodLifecycleState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PodLifecycleState::Populated => write!(f, "populated"),
            PodLifecycleState::Registered => write!(f, "registered"),
            PodLifecycleState::Activated => write!(f, "activated"),
            PodLifecycleState::Deactivated => write!(f, "deactivated"),
        }
    }
}

/// Agent pod unique identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PodID(pub uuid::Uuid);

impl PodID {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for PodID {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for PodID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Agent type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentType {
    /// Bot — Process execution, machine-to-machine (A2A)
    Bot,
    /// Replicant — Human assistance, human-to-agent (H2A)
    Replicant,
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentType::Bot => write!(f, "Bot"),
            AgentType::Replicant => write!(f, "Replicant"),
        }
    }
}

/// Agent persona definition (from YAML)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPersona {
    /// Agent identity
    pub agent: AgentIdentity,
    /// Agent charter (purpose and scope)
    pub charter: AgentCharter,
    /// Capabilities this agent requires
    pub capabilities: Vec<String>,
    /// Rights (access permissions)
    pub rights: Vec<AccessRight>,
    /// Responsibilities (obligations)
    pub responsibilities: Vec<String>,
    /// Default visibility for artifacts
    pub visibility: VisibilitySettings,
    /// Cached WebID (derived deterministically from persona)
    #[serde(skip)]
    cached_webid: Option<WebID>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentIdentity {
    pub name: String,
    #[serde(rename = "type")]
    pub agent_type: AgentType,
    #[serde(default = "default_version")]
    pub version: String,
}

fn default_version() -> String {
    "0.1.0".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCharter {
    pub description: String,
    pub editor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessRight {
    pub read: Option<String>,
    pub write: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisibilitySettings {
    #[serde(default = "default_public_visibility")]
    pub default: hkask_types::Visibility,
    #[serde(default = "default_private_visibility")]
    pub episodic_override: hkask_types::Visibility,
}

fn default_public_visibility() -> hkask_types::Visibility {
    hkask_types::Visibility::Public
}

fn default_private_visibility() -> hkask_types::Visibility {
    hkask_types::Visibility::Private
}

impl AgentPersona {
    /// Create a new AgentPersona with deterministic WebID
    pub fn new(
        agent: AgentIdentity,
        charter: AgentCharter,
        capabilities: Vec<String>,
        rights: Vec<AccessRight>,
        responsibilities: Vec<String>,
        visibility: VisibilitySettings,
    ) -> Self {
        let mut persona = Self {
            agent,
            charter,
            capabilities,
            rights,
            responsibilities,
            visibility,
            cached_webid: None,
        };
        // Compute and cache WebID
        let canonical = serde_json::to_string(&persona.agent).unwrap_or_default();
        persona.cached_webid = Some(WebID::from_persona(canonical.as_bytes()));
        persona
    }

    /// Parse agent persona from YAML string
    pub fn from_yaml(yaml: &str) -> Result<Self, AgentPodError> {
        let mut persona: Self = serde_yaml::from_str(yaml)
            .map_err(|e| AgentPodError::PersonaParseError(e.to_string()))?;

        // Compute and cache WebID
        let canonical = serde_json::to_string(&persona.agent).unwrap_or_default();
        persona.cached_webid = Some(WebID::from_persona(canonical.as_bytes()));

        Ok(persona)
    }

    /// Get the agent's WebID (derived deterministically from persona)
    pub fn webid(&self) -> WebID {
        self.cached_webid.unwrap_or_else(|| {
            let canonical = serde_json::to_string(&self.agent).unwrap_or_default();
            WebID::from_persona(canonical.as_bytes())
        })
    }

    /// Get capabilities as CapabilityResource enums
    pub fn capability_resources(&self) -> Vec<CapabilityResource> {
        self.capabilities
            .iter()
            .filter_map(|cap| {
                if cap.starts_with("tool:") {
                    Some(CapabilityResource::Tool)
                } else if cap.starts_with("template:") {
                    Some(CapabilityResource::Template)
                } else if cap.starts_with("memory:") {
                    Some(CapabilityResource::Cascade)
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Template crate structure (loaded from Git CAS)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TemplateCrate {
    /// Crate name
    pub name: String,
    /// Git SHA (pinned version)
    pub git_sha: String,
    /// Agent persona YAML content
    pub persona_yaml: String,
    /// Dispatch manifest YAML content
    pub dispatch_manifest_yaml: String,
    /// Template files (path -> content)
    pub templates: Vec<TemplateFile>,
    /// hLexicon terms used
    pub hlexicon_terms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateFile {
    pub path: String,
    pub content: String,
    pub template_type: String, // Prompt, Process, Cognition
}

/// Agent Pod — Runtime container for ACP agents
pub struct AgentPod {
    /// Unique pod identifier
    pub id: PodID,
    /// Agent's WebID
    pub webid: WebID,
    /// Agent type (Bot or Replicant)
    pub agent_type: AgentType,
    /// Agent persona
    pub persona: AgentPersona,
    /// Template crate reference
    pub template_crate: TemplateCrate,
    /// Primary capability token
    pub capability_token: CapabilityToken,
    /// Current lifecycle state
    pub state: PodLifecycleState,
    /// Pod creation timestamp (Unix epoch)
    pub created_at: i64,
    /// Maximum attenuation level for delegation
    pub max_attenuation: u8,
    /// Keystore for secure secret storage
    pub keystore: Keychain,
    /// Sovereignty checker for this pod
    pub sovereignty_checker: SovereigntyChecker,
}

/// Maximum attenuation level (OCAP security limit)
pub const MAX_ATTENUATION_LEVEL: u8 = 7;

/// Agent pod error types
#[derive(Debug, Error)]
pub enum AgentPodError {
    #[error("Failed to parse agent persona: {0}")]
    PersonaParseError(String),

    #[error("Failed to load template crate: {0}")]
    CrateLoadError(String),

    #[error("ACP registration failed: {0}")]
    ACPRegistrationError(String),

    #[error("MCP access grant failed: {0}")]
    MCPAccessError(String),

    #[error("Capability attenuation limit exceeded")]
    AttenuationLimitExceeded,

    #[error("Invalid lifecycle transition: {0} -> {1}")]
    InvalidStateTransition(PodLifecycleState, PodLifecycleState),

    #[error("Pod is not in required state: expected {expected}, actual {actual}")]
    StateMismatch {
        expected: PodLifecycleState,
        actual: PodLifecycleState,
    },

    #[error("CNS event emission failed: {0}")]
    CNSEmissionError(String),

    #[error("Keystore error: {0}")]
    KeystoreError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Clock error: {0}")]
    ClockError(String),

    #[error("Capability denied: token does not grant {resource:?} {action:?}")]
    CapabilityDenied {
        resource: CapabilityResource,
        action: CapabilityAction,
    },

    #[error("Inference port unavailable: {0}")]
    InferenceUnavailable(String),

    #[error("Memory operation failed: {0}")]
    MemoryError(String),

    #[error("Tool invocation failed: {0}")]
    ToolError(String),
}

/// Result type for agent pod operations
pub type AgentPodResult<T> = Result<T, AgentPodError>;

impl AgentPod {
    /// Instantiate a new agent pod from a template crate
    ///
    /// # Arguments
    /// * `crate_name` — Name of the template crate to load
    /// * `persona` — Agent persona definition
    /// * `git` — Git CAS port for loading crate contents
    ///
    /// # Returns
    /// * `Ok(AgentPod)` — Pod created in `Populated` state
    /// * `Err(AgentPodError)` — Failed to load crate or parse persona
    pub fn new(
        crate_name: &str,
        persona: &AgentPersona,
        git: &dyn GitCASPort,
    ) -> AgentPodResult<Self> {
        let template_crate = git
            .load_template_crate(crate_name)
            .map_err(|e| AgentPodError::CrateLoadError(e.to_string()))?;

        // Initialize keystore for secure secret storage
        let keystore = Keychain::default();

        // Retrieve or generate OCAP secret from keystore
        let ocap_secret = get_or_create_ocap_secret(&keystore, &persona.webid())?;

        // Use first capability from persona, or default to "tool:execute"
        let first_capability = persona
            .capabilities
            .first()
            .cloned()
            .unwrap_or_else(|| "tool:execute".to_string());

        let capability_token = CapabilityToken::new(
            CapabilityResource::Tool,
            first_capability,
            CapabilityAction::Execute,
            WebID::new(),
            persona.webid(),
            ocap_secret.as_bytes(),
        );

        // Initialize sovereignty checker for this pod
        let sovereignty_checker = SovereigntyChecker::new(persona.webid());

        Ok(Self {
            id: PodID::new(),
            webid: persona.webid(),
            agent_type: persona.agent.agent_type,
            persona: persona.clone(),
            template_crate,
            capability_token,
            state: PodLifecycleState::Populated,
            created_at: current_timestamp()?,
            max_attenuation: MAX_ATTENUATION_LEVEL,
            keystore,
            sovereignty_checker,
        })
    }

    /// Register the pod with the ACP runtime
    ///
    /// Transitions state: `Populated` → `Registered`
    ///
    /// # Arguments
    /// * `acp` — ACP runtime port for agent registration
    /// * `cns` — CNS span emitter for lifecycle events
    ///
    /// # Returns
    /// * `Ok(())` — Registration successful
    /// * `Err(AgentPodError)` — ACP registration failed
    pub async fn register(
        &mut self,
        acp: &dyn crate::ports::AcpPort,
        cns: &dyn CnsEmit,
    ) -> AgentPodResult<()> {
        if self.state != PodLifecycleState::Populated {
            return Err(AgentPodError::InvalidStateTransition(
                self.state,
                PodLifecycleState::Registered,
            ));
        }

        let capabilities: Vec<String> = self.persona.capabilities.clone();
        let agent_type = self.agent_type.to_string();
        let token = acp
            .register_agent(self.webid, &agent_type, capabilities)
            .await
            .map_err(|e| AgentPodError::ACPRegistrationError(e.to_string()))?;

        self.capability_token = token;
        self.state = PodLifecycleState::Registered;

        cns.emit_event(
            "cns.agent_pod.registered",
            "registered",
            &serde_json::json!({
                "pod_id": self.id.to_string(),
                "webid": self.webid.to_string(),
                "agent_type": self.agent_type.to_string(),
            }),
            1.0,
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
    /// * `cns` — CNS span emitter for lifecycle events
    ///
    /// # Returns
    /// * `Ok(())` — Activation successful
    /// * `Err(AgentPodError)` — MCP access grant failed
    pub fn activate(&mut self, mcp: &dyn MCPRuntimePort, cns: &dyn CnsEmit) -> AgentPodResult<()> {
        if self.state != PodLifecycleState::Registered {
            return Err(AgentPodError::InvalidStateTransition(
                self.state,
                PodLifecycleState::Activated,
            ));
        }

        mcp.grant_tool_access(self.capability_token.clone())
            .map_err(|e| AgentPodError::MCPAccessError(e.to_string()))?;

        self.state = PodLifecycleState::Activated;

        cns.emit_event(
            "cns.agent_pod.activated",
            "activated",
            &serde_json::json!({
                "pod_id": self.id.to_string(),
                "webid": self.webid.to_string(),
                "mcp_access": true,
            }),
            1.0,
        );

        info!("Agent pod {} activated for A2A communication", self.id);
        Ok(())
    }

    /// Deactivate the pod and revoke capabilities
    ///
    /// Transitions state: `Activated` → `Deactivated`
    ///
    /// # Arguments
    /// * `cns` — CNS span emitter for lifecycle events
    ///
    /// # Returns
    /// * `Ok(())` — Deactivation successful
    pub fn deactivate(&mut self, cns: &dyn CnsEmit) -> AgentPodResult<()> {
        if self.state != PodLifecycleState::Activated {
            return Err(AgentPodError::InvalidStateTransition(
                self.state,
                PodLifecycleState::Deactivated,
            ));
        }

        self.state = PodLifecycleState::Deactivated;

        cns.emit_event(
            "cns.agent_pod.deactivated",
            "deactivated",
            &serde_json::json!({
                "pod_id": self.id.to_string(),
                "webid": self.webid.to_string(),
                "capabilities_revoked": true,
            }),
            1.0,
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
    /// * `Ok(CapabilityToken)` — Attenuated child token
    /// * `Err(AgentPodError)` — Attenuation limit exceeded or keystore error
    pub fn delegate(
        &self,
        new_holder: WebID,
        current_time: i64,
    ) -> AgentPodResult<CapabilityToken> {
        // Check attenuation limit
        if self.capability_token.attenuation_level >= self.max_attenuation {
            return Err(AgentPodError::AttenuationLimitExceeded);
        }

        // Retrieve OCAP secret from keystore for attenuation
        let ocap_secret = get_or_create_ocap_secret(&self.keystore, &self.webid)?;

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
        data_category: &str,
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

    /// Get sovereignty checker reference
    pub fn sovereignty_checker(&self) -> &SovereigntyChecker {
        &self.sovereignty_checker
    }

    /// Get mutable sovereignty checker reference
    pub fn sovereignty_checker_mut(&mut self) -> &mut SovereigntyChecker {
        &mut self.sovereignty_checker
    }
}

fn current_timestamp() -> Result<i64, AgentPodError> {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .map_err(|e| AgentPodError::ClockError(e.to_string()))
}

/// Get or create OCAP secret for a WebID from the keystore
///
/// This function retrieves an existing OCAP secret from the keystore,
/// or generates and stores a new one if it doesn't exist.
///
/// # Arguments
/// * `keystore` — Keychain instance for secure storage
/// * `webid` — WebID to associate with the secret
///
/// # Returns
/// * `Ok(Zeroizing<String>)` — OCAP secret (zeroized for security)
/// * `Err(AgentPodError)` — Keystore error
fn get_or_create_ocap_secret(
    keystore: &Keychain,
    webid: &WebID,
) -> AgentPodResult<Zeroizing<String>> {
    // Try to retrieve existing secret
    match keystore.retrieve(webid) {
        Ok(secret) => Ok(Zeroizing::new(secret)),
        Err(_) => {
            // Generate new random secret
            let secret = generate_secure_ocap_secret();

            // Store in keystore
            keystore
                .store(webid, &secret)
                .map_err(|e| AgentPodError::KeystoreError(e.to_string()))?;

            Ok(Zeroizing::new(secret))
        }
    }
}

/// Generate a secure random OCAP secret
///
/// Creates a 32-byte random secret encoded as hex string (64 characters).
/// This provides 256 bits of entropy for cryptographic security.
fn generate_secure_ocap_secret() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

// Port traits are now defined in crate::ports — re-exported here for backward compatibility.
// MCPRuntimePort → crate::ports::MCPRuntimePort
// GitCASPort → crate::ports::GitCASPort
// MemoryStoragePort → crate::ports::MemoryStoragePort

/// Pod Manager — Manages collection of agent pods
///
/// The PodManager provides centralized lifecycle management for all agent pods
/// in the hKask system. It handles:
/// - Pod creation from template crates
/// - Pod activation/deactivation
/// - Status queries
/// - Listing all pods
/// - Inference access via InferencePort
pub struct PodManager {
    pods: Arc<RwLock<HashMap<PodID, AgentPod>>>,
    _keystore: Keychain,
    git_cas: Arc<dyn GitCASPort>,
    acp_runtime: Arc<dyn crate::ports::AcpPort + Send + Sync>,
    cns_emitter: Arc<dyn hkask_cns::CnsEmit + Send + Sync>,
    mcp_runtime: Arc<dyn MCPRuntimePort>,
    memory_storage: Arc<dyn MemoryStoragePort>,
    security_context: SecurityContext,
    inference_port: Option<Arc<dyn hkask_templates::InferencePort>>,
}

/// Pod status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodStatus {
    pub pod_id: String,
    pub name: Option<String>,
    pub state: PodLifecycleState,
    pub webid: String,
    pub agent_type: AgentType,
    pub template: String,
    pub created_at: i64,
}

impl PodManager {
    /// Create a new pod manager with trait-object adapters
    pub fn new(
        git_cas: Arc<dyn GitCASPort>,
        acp_runtime: Arc<dyn crate::ports::AcpPort + Send + Sync>,
        cns_emitter: Arc<dyn hkask_cns::CnsEmit + Send + Sync>,
        mcp_runtime: Arc<dyn MCPRuntimePort>,
        memory_storage: Arc<dyn MemoryStoragePort>,
    ) -> Self {
        Self {
            pods: Arc::new(RwLock::new(HashMap::new())),
            _keystore: Keychain::default(),
            git_cas,
            acp_runtime,
            cns_emitter,
            mcp_runtime,
            memory_storage,
            security_context: SecurityContext::default(),
            inference_port: None,
        }
    }

    /// Create a new pod manager with inference port
    pub fn with_inference(
        git_cas: Arc<dyn GitCASPort>,
        acp_runtime: Arc<dyn crate::ports::AcpPort + Send + Sync>,
        cns_emitter: Arc<dyn hkask_cns::CnsEmit + Send + Sync>,
        mcp_runtime: Arc<dyn MCPRuntimePort>,
        memory_storage: Arc<dyn MemoryStoragePort>,
        inference_port: Arc<dyn hkask_templates::InferencePort>,
    ) -> Self {
        Self {
            pods: Arc::new(RwLock::new(HashMap::new())),
            _keystore: Keychain::default(),
            git_cas,
            acp_runtime,
            cns_emitter,
            mcp_runtime,
            memory_storage,
            security_context: SecurityContext::default(),
            inference_port: Some(inference_port),
        }
    }

    /// Get the inference port if available
    pub fn inference_port(&self) -> Option<Arc<dyn hkask_templates::InferencePort>> {
        self.inference_port.clone()
    }

    /// Create a new pod manager with mock adapters for testing
    pub fn new_mock() -> Self {
        Self {
            pods: Arc::new(RwLock::new(HashMap::new())),
            _keystore: Keychain::default(),
            git_cas: Arc::new(GitCasAdapter::from_path(PathBuf::from("/tmp/hkask-mock"))),
            acp_runtime: Arc::new(crate::acp::AcpRuntime::default()),
            cns_emitter: Arc::new(CnsEmitterAdapter::new(WebID::new())),
            mcp_runtime: Arc::new(McpRuntimeAdapter::new()),
            memory_storage: Arc::new(
                MemoryStorageAdapter::in_memory()
                    .expect("In-memory storage initialization should never fail"),
            ),
            security_context: SecurityContext::default(),
            inference_port: None,
        }
    }

    #[cfg(test)]
    fn create_test_pod_manager_with_templates() -> PodManager {
        Self {
            pods: Arc::new(RwLock::new(HashMap::new())),
            _keystore: Keychain::default(),
            git_cas: Arc::new(GitCasAdapter::from_path(PathBuf::from("./registry/templates"))),
            acp_runtime: Arc::new(crate::acp::AcpRuntime::default()),
            cns_emitter: Arc::new(CnsEmitterAdapter::new(WebID::new())),
            mcp_runtime: Arc::new(McpRuntimeAdapter::new()),
            memory_storage: Arc::new(
                MemoryStorageAdapter::in_memory()
                    .expect("In-memory storage initialization should never fail"),
            ),
            security_context: SecurityContext::default(),
            inference_port: None,
        }
    }
}

/// Builder for constructing [`PodManager`] with explicit adapter configuration
///
/// # Example
///
/// ```rust,no_run
/// use hkask_agents::pod::PodManagerBuilder;
/// use hkask_agents::adapters::git_cas::GitCasAdapter;
/// use std::path::PathBuf;
/// use std::sync::Arc;
///
/// let pod_manager = PodManagerBuilder::new()
///     .git_cas(Arc::new(GitCasAdapter::from_path(PathBuf::from("./registry/templates"))))
///     .with_in_memory_storage()
///     .build();
/// ```
pub struct PodManagerBuilder {
    git_cas: Option<Arc<dyn GitCASPort>>,
    acp_runtime: Option<Arc<dyn crate::ports::AcpPort + Send + Sync>>,
    cns_emitter: Option<Arc<dyn hkask_cns::CnsEmit + Send + Sync>>,
    mcp_runtime: Option<Arc<dyn MCPRuntimePort>>,
    memory_storage: Option<Arc<dyn MemoryStoragePort>>,
    security_context: Option<SecurityContext>,
    inference_port: Option<Arc<dyn hkask_templates::InferencePort>>,
}

impl PodManagerBuilder {
    pub fn new() -> Self {
        Self {
            git_cas: None,
            acp_runtime: None,
            cns_emitter: None,
            mcp_runtime: None,
            memory_storage: None,
            security_context: None,
            inference_port: None,
        }
    }

    pub fn git_cas(mut self, adapter: Arc<dyn GitCASPort>) -> Self {
        self.git_cas = Some(adapter);
        self
    }

    pub fn git_cas_from_path<P: Into<PathBuf>>(self, path: P) -> Self {
        self.git_cas(Arc::new(GitCasAdapter::from_path(path.into())))
    }

    pub fn acp_runtime(mut self, adapter: Arc<dyn crate::ports::AcpPort + Send + Sync>) -> Self {
        self.acp_runtime = Some(adapter);
        self
    }

    pub fn cns_emitter(mut self, adapter: Arc<dyn hkask_cns::CnsEmit + Send + Sync>) -> Self {
        self.cns_emitter = Some(adapter);
        self
    }

    pub fn mcp_runtime(mut self, adapter: Arc<dyn MCPRuntimePort>) -> Self {
        self.mcp_runtime = Some(adapter);
        self
    }

    pub fn memory_storage(mut self, adapter: Arc<dyn MemoryStoragePort>) -> Self {
        self.memory_storage = Some(adapter);
        self
    }

    pub fn inference_port(mut self, adapter: Arc<dyn hkask_templates::InferencePort>) -> Self {
        self.inference_port = Some(adapter);
        self
    }

    pub fn with_in_memory_storage(self) -> Self {
        self.memory_storage(Arc::new(
            MemoryStorageAdapter::in_memory()
                .expect("In-memory storage initialization should never fail"),
        ))
    }

    pub fn with_encrypted_storage<P: AsRef<std::path::Path>>(
        self,
        path: P,
        passphrase: &str,
    ) -> Self {
        let path_str = path
            .as_ref()
            .to_str()
            .expect("Storage path must be valid UTF-8");
        self.memory_storage(Arc::new(
            MemoryStorageAdapter::from_path(path_str, passphrase)
                .expect("Encrypted storage initialization should succeed"),
        ))
    }

    pub fn security_context(mut self, context: SecurityContext) -> Self {
        self.security_context = Some(context);
        self
    }

    pub fn build(self) -> PodManager {
        let mut manager = PodManager::new(
            self.git_cas
                .unwrap_or_else(|| Arc::new(GitCasAdapter::from_path(PathBuf::from("./registry/templates")))),
            self.acp_runtime
                .unwrap_or_else(|| Arc::new(crate::acp::AcpRuntime::default())),
            self.cns_emitter
                .unwrap_or_else(|| Arc::new(CnsEmitterAdapter::new(WebID::new()))),
            self.mcp_runtime
                .unwrap_or_else(|| Arc::new(McpRuntimeAdapter::new())),
            self.memory_storage.unwrap_or_else(|| {
                Arc::new(
                    MemoryStorageAdapter::in_memory()
                        .expect("In-memory storage initialization should never fail"),
                )
            }),
        );
        manager.inference_port = self.inference_port;
        if let Some(ctx) = self.security_context {
            manager.security_context = ctx;
        }
        manager
    }
}

impl Default for PodManagerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl PodManager {
    /// Create a new pod from a template crate
    ///
    /// # Arguments
    /// * `template_name` — Name of the template crate
    /// * `persona` — Agent persona definition
    /// * `name` — Optional pod name (defaults to UUID)
    ///
    /// # Returns
    /// * `Ok(PodID)` — Pod created successfully
    /// * `Err(AgentPodError)` — Failed to create pod
    pub async fn create_pod(
        &self,
        template_name: &str,
        persona: &AgentPersona,
        name: Option<String>,
    ) -> AgentPodResult<PodID> {
        // Rate limit pod creation
        let rate_key = format!("pod_creation:{}", template_name);
        self.security_context
            .rate_limiter
            .acquire(&rate_key, 1.0)
            .await
            .map_err(|e| match e {
                crate::security::ValidationError::RateLimitExceeded => {
                    AgentPodError::ACPRegistrationError("Rate limit exceeded".to_string())
                }
                _ => AgentPodError::ACPRegistrationError(e.to_string()),
            })?;

        // Validate persona input
        let input = AgentPersonaInput {
            name: persona.agent.name.clone(),
            agent_type: persona.agent.agent_type.to_string().to_lowercase(),
            version: persona.agent.version.clone(),
            description: persona.charter.description.clone(),
            editor: persona.charter.editor.clone(),
            capabilities: persona.capabilities.clone(),
        };

        input
            .validate(&input)
            .map_err(|e| AgentPodError::PersonaParseError(e.to_string()))?;

        let pod = AgentPod::new(template_name, persona, self.git_cas.as_ref())?;
        let pod_id = pod.id;

        let mut pods = self.pods.write().await;
        pods.insert(pod_id, pod);

        info!(
            target: "hkask.pod",
            pod_id = %pod_id,
            template = %template_name,
            name = ?name,
            "Pod created"
        );

        Ok(pod_id)
    }

    /// Activate a pod for A2A communication
    pub async fn activate_pod(&self, pod_id: &PodID) -> AgentPodResult<()> {
        // Phase 1: Extract registration data while holding the guard
        let registration_data = {
            let pods = self.pods.read().await;
            let pod = pods
                .get(pod_id)
                .ok_or_else(|| AgentPodError::ACPRegistrationError("Pod not found".to_string()))?;

            if pod.state() == PodLifecycleState::Populated {
                Some((
                    pod.webid,
                    pod.agent_type.to_string(),
                    pod.persona.capabilities.clone(),
                ))
            } else {
                None
            }
        }; // Guard dropped here

        // Phase 2: Async ACP registration without holding the lock
        let token = if let Some((webid, agent_type, capabilities)) = registration_data {
            Some(
                self.acp_runtime
                    .register_agent(webid, &agent_type, capabilities)
                    .await
                    .map_err(|e| AgentPodError::ACPRegistrationError(e.to_string()))?,
            )
        } else {
            None
        };

        // Phase 3: Apply result and activate MCP while holding write guard
        let mut pods = self.pods.write().await;
        let pod = pods
            .get_mut(pod_id)
            .ok_or_else(|| AgentPodError::ACPRegistrationError("Pod not found".to_string()))?;

        if let Some(token) = token {
            pod.capability_token = token;
            pod.state = PodLifecycleState::Registered;

            self.cns_emitter.emit_event(
                "cns.agent_pod.registered",
                "registered",
                &serde_json::json!({
                    "pod_id": pod.id.to_string(),
                    "webid": pod.webid.to_string(),
                    "agent_type": pod.agent_type.to_string(),
                }),
                1.0,
            );

            info!("Agent pod {} registered with ACP", pod.id);
        }

        pod.activate(self.mcp_runtime.as_ref(), self.cns_emitter.as_ref())?;

        // Persist activation event to memory storage
        let event = serde_json::json!({
            "entity": pod.webid.to_string(),
            "attribute": "lifecycle_event",
            "value": {
                "event": "activated",
                "pod_id": pod.id.to_string(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }
        });

        let _ = self.memory_storage.store_artifact(
            pod.webid,
            "episodic_triple",
            event,
            "private",
            &pod.capability_token,
        );

        info!(
            target: "hkask.pod",
            pod_id = %pod_id,
            "Pod activated"
        );

        Ok(())
    }

    /// Deactivate a pod
    pub async fn deactivate_pod(&self, pod_id: &PodID) -> AgentPodResult<()> {
        let mut pods = self.pods.write().await;
        let pod = pods
            .get_mut(pod_id)
            .ok_or_else(|| AgentPodError::ACPRegistrationError("Pod not found".to_string()))?;

        let token_id = pod.capability_token.id.clone();
        let webid = pod.webid;

        pod.deactivate(self.cns_emitter.as_ref())?;

        // W6: Revoke capability token on deactivation
        if let Err(e) = self.acp_runtime.revoke_capability(&token_id, &webid).await {
            tracing::warn!(
                target: "hkask.pod",
                pod_id = %pod_id,
                token_id = %token_id,
                error = %e,
                "Failed to revoke capability token on deactivation (pod is still deactivated)"
            );
            self.cns_emitter.emit_event(
                "cns.agent_pod.revocation_warning",
                "revocation_warning",
                &serde_json::json!({
                    "pod_id": pod_id.to_string(),
                    "token_id": token_id,
                    "error": e.to_string(),
                }),
                0.8,
            );
        }

        // Persist deactivation event to memory storage
        let event = serde_json::json!({
            "entity": pod.webid.to_string(),
            "attribute": "lifecycle_event",
            "value": {
                "event": "deactivated",
                "pod_id": pod.id.to_string(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }
        });

        let _ = self.memory_storage.store_artifact(
            pod.webid,
            "episodic_triple",
            event,
            "private",
            &pod.capability_token,
        );

        info!(
            target: "hkask.pod",
            pod_id = %pod_id,
            "Pod deactivated"
        );

        Ok(())
    }

    /// Recall lifecycle events for a pod
    pub async fn recall_pod_events(
        &self,
        pod_id: &PodID,
    ) -> AgentPodResult<Vec<serde_json::Value>> {
        let pods = self.pods.read().await;
        let pod = pods
            .get(pod_id)
            .ok_or_else(|| AgentPodError::ACPRegistrationError("Pod not found".to_string()))?;

        let results = self.memory_storage
            .recall(&pod.webid.to_string(), &pod.capability_token)
            .map_err(|e| AgentPodError::StorageError(e.to_string()))?;

        Ok(results)
    }

    /// Get pod status
    pub async fn get_pod_status(&self, pod_id: &PodID) -> AgentPodResult<PodStatus> {
        let pods = self.pods.read().await;
        let pod = pods
            .get(pod_id)
            .ok_or_else(|| AgentPodError::ACPRegistrationError("Pod not found".to_string()))?;

        Ok(PodStatus {
            pod_id: pod.id.to_string(),
            name: Some(pod.persona.agent.name.clone()),
            state: pod.state,
            webid: pod.webid.to_string(),
            agent_type: pod.agent_type,
            template: pod.template_crate.name.clone(),
            created_at: pod.created_at,
        })
    }

    pub async fn list_pods(&self) -> AgentPodResult<Vec<PodStatus>> {
        let pods = self.pods.read().await;
        let statuses = pods
            .values()
            .map(|pod| PodStatus {
                pod_id: pod.id.to_string(),
                name: Some(pod.persona.agent.name.clone()),
                state: pod.state,
                webid: pod.webid.to_string(),
                agent_type: pod.agent_type,
                template: pod.template_crate.name.clone(),
                created_at: pod.created_at,
            })
            .collect();

        Ok(statuses)
    }

    /// Get a reference to the ACP runtime port
    pub fn acp_runtime(&self) -> Arc<dyn crate::ports::AcpPort + Send + Sync> {
        Arc::clone(&self.acp_runtime)
    }
}

impl Default for PodManager {
    fn default() -> Self {
        Self::new_mock()
    }
}

/// PodContext — Runtime context for an active pod
///
/// Provides access to all ports (inference, memory, MCP, CNS) for a specific pod.
/// This is the unit of access that enforces the pod invariant: all interactions
/// with memory, inference, and tools must go through a pod context.
pub struct PodContext {
    pub pod_id: PodID,
    pub webid: WebID,
    pub capability_token: CapabilityToken,
    inference_port: Option<Arc<dyn hkask_templates::InferencePort>>,
    memory_storage: Arc<dyn MemoryStoragePort>,
    mcp_runtime: Arc<dyn MCPRuntimePort>,
    cns_emitter: Arc<dyn hkask_cns::CnsEmit + Send + Sync>,
}

impl PodContext {
    pub async fn from_manager(manager: &PodManager, pod_id: &PodID) -> AgentPodResult<Self> {
        let pods = manager.pods.read().await;
        let pod = pods
            .get(pod_id)
            .ok_or_else(|| AgentPodError::ACPRegistrationError("Pod not found".to_string()))?;

        if pod.state != PodLifecycleState::Activated {
            return Err(AgentPodError::ACPRegistrationError(
                "Pod must be activated before creating context".to_string(),
            ));
        }

        Ok(Self {
            pod_id: *pod_id,
            webid: pod.webid,
            capability_token: pod.capability_token.clone(),
            inference_port: manager.inference_port.clone(),
            memory_storage: Arc::clone(&manager.memory_storage),
            mcp_runtime: Arc::clone(&manager.mcp_runtime),
            cns_emitter: Arc::clone(&manager.cns_emitter),
        })
    }

    fn require_capability(
        &self,
        resource: CapabilityResource,
        resource_id: &str,
        action: CapabilityAction,
    ) -> AgentPodResult<()> {
        if !self.capability_token.is_valid_for(resource, resource_id, action) {
            return Err(AgentPodError::CapabilityDenied { resource, action });
        }
        Ok(())
    }

    pub fn inference_port(&self) -> AgentPodResult<Arc<dyn hkask_templates::InferencePort>> {
        self.require_capability(CapabilityResource::Template, "inference", CapabilityAction::Render)?;
        self.inference_port
            .clone()
            .ok_or_else(|| AgentPodError::InferenceUnavailable("No inference port configured".to_string()))
    }

    pub async fn recall_memory(&self, query: &str) -> AgentPodResult<Vec<serde_json::Value>> {
        self.require_capability(CapabilityResource::Manifest, "memory", CapabilityAction::Read)?;
        self.memory_storage
            .recall(query, &self.capability_token)
            .map_err(|e| AgentPodError::MemoryError(e.to_string()))
    }

    pub async fn store_memory(
        &self,
        artifact_type: &str,
        content: serde_json::Value,
        visibility: &str,
    ) -> AgentPodResult<String> {
        self.require_capability(CapabilityResource::Manifest, "memory", CapabilityAction::Write)?;
        self.memory_storage
            .store_artifact(
                self.webid,
                artifact_type,
                content,
                visibility,
                &self.capability_token,
            )
            .map_err(|e| AgentPodError::MemoryError(e.to_string()))
    }

    pub fn invoke_tool(
        &self,
        tool_name: &str,
        input: serde_json::Value,
    ) -> AgentPodResult<serde_json::Value> {
        self.require_capability(CapabilityResource::Tool, tool_name, CapabilityAction::Execute)?;
        self.emit_span(
            &format!("cns.tool.{}", tool_name),
            "invoked",
            serde_json::json!({ "input_keys": input.as_object().map(|o| o.keys().collect::<Vec<_>>()) }),
        );
        let result = self
            .mcp_runtime
            .invoke_tool(tool_name, input, &self.capability_token)
            .map_err(|e| AgentPodError::ToolError(e.to_string()));
        match &result {
            Ok(_) => self.emit_span(
                &format!("cns.tool.{}.completed", tool_name),
                "completed",
                serde_json::json!({}),
            ),
            Err(_) => self.emit_span(
                &format!("cns.tool.{}.failed", tool_name),
                "failed",
                serde_json::json!({}),
            ),
        }
        result
    }

    pub fn emit_span(&self, span_type: &str, action: &str, data: serde_json::Value) {
        self.cns_emitter.emit_event(
            span_type,
            "action",
            &serde_json::json!({
                "pod_id": self.pod_id.to_string(),
                "webid": self.webid.to_string(),
                "action": action,
                "data": data,
            }),
            1.0,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

<<<<<<< HEAD
    #[test]
    fn test_persona_webid_deterministic() {
        let yaml = r#"
=======
    pub struct MockMCPRuntime;
    impl MCPRuntimePort for MockMCPRuntime {
        fn grant_tool_access(&self, _token: CapabilityToken) -> Result<(), String> {
            Ok(())
        }

        fn invoke_tool(
            &self,
            _tool_name: &str,
            _input: serde_json::Value,
            _token: &CapabilityToken,
        ) -> Result<serde_json::Value, String> {
            Ok(serde_json::json!({"result": "success"}))
        }
    }

    pub struct MockCNSSpan;
    impl CNSSpanPort for MockCNSSpan {
        fn emit_event(
            &self,
            _span: &str,
            _phase: &str,
            _observation: &serde_json::Value,
            _confidence: f64,
        ) {
            // No-op for tests
        }
    }

    pub struct MockGitCAS;
    impl GitCASPort for MockGitCAS {
        fn load_template_crate(&self, _crate_name: &str) -> Result<TemplateCrate, String> {
            Ok(TemplateCrate {
                name: "test-crate".to_string(),
                git_sha: "abc123".to_string(),
                persona_yaml: String::new(),
                dispatch_manifest_yaml: String::new(),
                templates: vec![],
                hlexicon_terms: vec![],
            })
        }

        fn resolve_sha(&self, _crate_name: &str) -> Result<String, String> {
            Ok("abc123".to_string())
        }
    }

    #[tokio::test]
    async fn test_pod_manager_security_context() {
        let manager = PodManager::new_mock();
        assert!(
            manager
                .security_context()
                .rate_limiter
                .get_available("test")
                .await
                > 0.0
        );
    }

    #[tokio::test]
    async fn test_pod_creation_validation() {
        let persona_yaml = r#"
>>>>>>> origin/main
agent:
  name: test-bot
  type: Bot
  version: "0.1.0"
charter:
  description: Test bot
  editor: test
capabilities:
  - "tool:execute"
rights: []
responsibilities: []
visibility:
  default: public
  episodic_override: private
"#;
        let persona1 = AgentPersona::from_yaml(yaml).unwrap();
        let persona2 = AgentPersona::from_yaml(yaml).unwrap();

        assert_eq!(
            persona1.webid(),
            persona2.webid(),
            "Same YAML should produce same WebID"
        );
    }

    #[test]
    fn test_persona_webid_different_for_different_agents() {
        let yaml1 = r#"
agent:
  name: bot-1
  type: Bot
  version: "0.1.0"
charter:
  description: Bot 1
  editor: test
capabilities: []
rights: []
responsibilities: []
visibility:
  default: public
  episodic_override: private
"#;
<<<<<<< HEAD
        let yaml2 = r#"
agent:
  name: bot-2
  type: Bot
  version: "0.1.0"
charter:
  description: Bot 2
  editor: test
capabilities: []
rights: []
responsibilities: []
visibility:
  default: public
  episodic_override: private
"#;
        let persona1 = AgentPersona::from_yaml(yaml1).unwrap();
        let persona2 = AgentPersona::from_yaml(yaml2).unwrap();

        assert_ne!(
            persona1.webid(),
            persona2.webid(),
            "Different agents should have different WebIDs"
        );
    }

    #[test]
    fn test_persona_webid_cached() {
        let yaml = r#"
agent:
  name: cached-bot
  type: Bot
  version: "0.1.0"
charter:
  description: Cached bot
  editor: test
capabilities: []
rights: []
responsibilities: []
visibility:
  default: public
  episodic_override: private
"#;
        let persona = AgentPersona::from_yaml(yaml).unwrap();
        let webid1 = persona.webid();
        let webid2 = persona.webid();
        let webid3 = persona.webid();

        assert_eq!(webid1, webid2);
        assert_eq!(webid2, webid3);
        assert_eq!(webid1, webid3);
=======
        let persona = AgentPersona::from_yaml(persona_yaml).unwrap();

        // Validate persona input
        let input = AgentPersonaInput {
            name: persona.agent.name.clone(),
            agent_type: persona.agent.agent_type.to_string().to_lowercase(),
            version: persona.agent.version.clone(),
            description: persona.charter.description.clone(),
            editor: persona.charter.editor.clone(),
            capabilities: persona.capabilities.clone(),
        };

        assert!(input.validate(&input).is_ok());
>>>>>>> origin/main
    }
}
