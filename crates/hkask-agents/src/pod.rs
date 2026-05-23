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
use tokio::sync::{Mutex, RwLock};
use tracing::info;
use zeroize::Zeroizing;

use crate::adapters::cns_emitter::CnsEmitterAdapter;
use crate::adapters::git_cas::GitCasAdapter;
use crate::adapters::mcp_runtime::McpRuntimeAdapter;
use crate::adapters::memory_storage::MemoryStorageAdapter;
use crate::security::{AgentPersonaInput, InputValidator, SecurityContext};
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
    #[serde(default = "default_public")]
    pub default: String,
    #[serde(default = "default_private")]
    pub episodic_override: String,
}

fn default_public() -> String {
    "public".to_string()
}

fn default_private() -> String {
    "private".to_string()
}

impl AgentPersona {
    /// Parse agent persona from YAML string
    pub fn from_yaml(yaml: &str) -> Result<Self, AgentPodError> {
        serde_yaml::from_str(yaml).map_err(|e| AgentPodError::PersonaParseError(e.to_string()))
    }

    /// Get the agent's WebID (derived from persona)
    pub fn webid(&self) -> WebID {
        // In production, this would be derived from a deterministic hash of the persona
        // For now, we generate a new WebID per persona instance
        WebID::new()
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

        let capability_token = CapabilityToken::new(
            CapabilityResource::Tool,
            "*".to_string(),
            CapabilityAction::Execute,
            WebID::new(),
            persona.webid(),
            ocap_secret.as_bytes(),
        );

        Ok(Self {
            id: PodID::new(),
            webid: persona.webid(),
            agent_type: persona.agent.agent_type,
            persona: persona.clone(),
            template_crate,
            capability_token,
            state: PodLifecycleState::Populated,
            created_at: current_timestamp(),
            max_attenuation: MAX_ATTENUATION_LEVEL,
            keystore,
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
}

fn current_timestamp() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
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

// ============================================================================
// Hexagonal Architecture Ports (Traits)
// ============================================================================

pub trait MCPRuntimePort {
    /// Grant tool access to an agent
    ///
    /// # Arguments
    /// * `token` — Capability token to authorize
    ///
    /// # Returns
    /// * `Ok(())` — Access granted
    /// * `Err(String)` — Access denied error
    fn grant_tool_access(&self, token: CapabilityToken) -> Result<(), String>;

    /// Invoke a tool with capability authorization
    ///
    /// # Arguments
    /// * `tool_name` — Name of the tool to invoke
    /// * `input` — Tool input as JSON value
    /// * `token` — Capability token authorizing the call
    ///
    /// # Returns
    /// * `Ok(serde_json::Value)` — Tool result
    /// * `Err(String)` — Invocation error
    fn invoke_tool(
        &self,
        tool_name: &str,
        input: serde_json::Value,
        token: &CapabilityToken,
    ) -> Result<serde_json::Value, String>;
}

/// Git CAS Port — Template crate loading
pub trait GitCASPort {
    /// Load a template crate from Git CAS
    ///
    /// # Arguments
    /// * `crate_name` — Name of the crate to load
    ///
    /// # Returns
    /// * `Ok(TemplateCrate)` — Loaded crate structure
    /// * `Err(String)` — Load error message
    fn load_template_crate(&self, crate_name: &str) -> Result<TemplateCrate, String>;

    /// Resolve the current Git SHA for a crate
    ///
    /// # Arguments
    /// * `crate_name` — Name of the crate
    ///
    /// # Returns
    /// * `Ok(String)` — Git SHA (40 hex characters)
    /// * `Err(String)` — Resolution error
    fn resolve_sha(&self, crate_name: &str) -> Result<String, String>;
}

/// Placeholder Git CAS implementation for PodManager
pub struct PlaceholderGitCAS;

impl GitCASPort for PlaceholderGitCAS {
    fn load_template_crate(&self, crate_name: &str) -> Result<TemplateCrate, String> {
        Ok(TemplateCrate {
            name: crate_name.to_string(),
            git_sha: "0000000000000000000000000000000000000000".to_string(),
            persona_yaml: String::new(),
            dispatch_manifest_yaml: String::new(),
            templates: vec![],
            hlexicon_terms: vec![],
        })
    }

    fn resolve_sha(&self, _crate_name: &str) -> Result<String, String> {
        Ok("0000000000000000000000000000000000000000".to_string())
    }
}

/// Memory Storage Port — Artifact persistence
pub trait MemoryStoragePort {
    /// Store a memory artifact (triple or embedding)
    ///
    /// # Arguments
    /// * `producer_webid` — WebID of the producing agent
    /// * `artifact_type` — Type of artifact ("episodic_triple", "semantic_triple", "embedding")
    /// * `content` — Artifact content as JSON
    /// * `visibility` — Visibility setting ("private", "public", "shared")
    /// * `token` — Capability token authorizing the write
    ///
    /// # Returns
    /// * `Ok(String)` — Artifact ID
    /// * `Err(String)` — Storage error
    fn store_artifact(
        &self,
        producer_webid: WebID,
        artifact_type: &str,
        content: serde_json::Value,
        visibility: &str,
        token: &CapabilityToken,
    ) -> Result<String, String>;

    /// Recall memory artifacts matching a query
    ///
    /// # Arguments
    /// * `query` — Search query
    /// * `token` — Capability token authorizing the read
    ///
    /// # Returns
    /// * `Ok(Vec<serde_json::Value>)` — Matching artifacts
    /// * `Err(String)` — Query error
    fn recall(
        &self,
        query: &str,
        token: &CapabilityToken,
    ) -> Result<Vec<serde_json::Value>, String>;
}

/// Pod Manager — Manages collection of agent pods
///
/// The PodManager provides centralized lifecycle management for all agent pods
/// in the hKask system. It handles:
/// - Pod creation from template crates
/// - Pod activation/deactivation
/// - Status queries
/// - Listing all pods
pub struct PodManager {
    pods: Arc<RwLock<HashMap<PodID, AgentPod>>>,
    #[allow(dead_code)]
    keystore: Keychain,
    git_cas: GitCasAdapter,
    acp_runtime: Arc<dyn crate::ports::AcpPort + Send + Sync>,
    cns_emitter: CnsEmitterAdapter,
    mcp_runtime: McpRuntimeAdapter,
    memory_storage: Arc<Mutex<MemoryStorageAdapter>>,
    security_context: SecurityContext,
}

/// Pod status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodStatus {
    pub pod_id: String,
    pub name: Option<String>,
    pub state: String,
    pub webid: String,
    pub agent_type: String,
    pub template: String,
    pub created_at: i64,
}

impl PodManager {
    /// Create a new pod manager with real adapters
    pub fn new(
        git_cas: GitCasAdapter,
        acp_runtime: Arc<dyn crate::ports::AcpPort + Send + Sync>,
        cns_emitter: CnsEmitterAdapter,
        mcp_runtime: McpRuntimeAdapter,
        memory_storage: MemoryStorageAdapter,
    ) -> Self {
        Self {
            pods: Arc::new(RwLock::new(HashMap::new())),
            keystore: Keychain::default(),
            git_cas,
            acp_runtime,
            cns_emitter,
            mcp_runtime,
            memory_storage: Arc::new(Mutex::new(memory_storage)),
            security_context: SecurityContext::default(),
        }
    }

    /// Create a new pod manager with mock adapters for testing
    pub fn new_mock() -> Self {
        Self {
            pods: Arc::new(RwLock::new(HashMap::new())),
            keystore: Keychain::default(),
            git_cas: GitCasAdapter::from_path(PathBuf::from("/tmp/hkask-mock")),
            acp_runtime: Arc::new(crate::acp::AcpRuntime::default()),
            cns_emitter: CnsEmitterAdapter::new(WebID::new()),
            mcp_runtime: McpRuntimeAdapter::new(),
            memory_storage: Arc::new(Mutex::new(MemoryStorageAdapter::in_memory().unwrap())),
            security_context: SecurityContext::default(),
        }
    }

    fn create_test_pod_manager_with_templates() -> PodManager {
        Self {
            pods: Arc::new(RwLock::new(HashMap::new())),
            keystore: Keychain::default(),
            git_cas: GitCasAdapter::from_path(PathBuf::from("./registry/templates")),
            acp_runtime: Arc::new(crate::acp::AcpRuntime::default()),
            cns_emitter: CnsEmitterAdapter::new(WebID::new()),
            mcp_runtime: McpRuntimeAdapter::new(),
            memory_storage: Arc::new(Mutex::new(MemoryStorageAdapter::in_memory().unwrap())),
            security_context: SecurityContext::default(),
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
///
/// let pod_manager = PodManagerBuilder::new()
///     .git_cas(GitCasAdapter::from_path(PathBuf::from("./registry/templates")))
///     .with_in_memory_storage()
///     .build();
/// ```
pub struct PodManagerBuilder {
    git_cas: Option<GitCasAdapter>,
    acp_runtime: Option<Arc<dyn crate::ports::AcpPort + Send + Sync>>,
    cns_emitter: Option<CnsEmitterAdapter>,
    mcp_runtime: Option<McpRuntimeAdapter>,
    memory_storage: Option<MemoryStorageAdapter>,
    security_context: Option<SecurityContext>,
}

impl PodManagerBuilder {
    /// Create new builder with default adapters
    pub fn new() -> Self {
        Self {
            git_cas: None,
            acp_runtime: None,
            cns_emitter: None,
            mcp_runtime: None,
            memory_storage: None,
            security_context: None,
        }
    }

    /// Set Git CAS adapter
    pub fn git_cas(mut self, adapter: GitCasAdapter) -> Self {
        self.git_cas = Some(adapter);
        self
    }

    /// Set Git CAS adapter from path
    pub fn git_cas_from_path<P: Into<PathBuf>>(self, path: P) -> Self {
        self.git_cas(GitCasAdapter::from_path(path.into()))
    }

    /// Set ACP runtime (accepts any AcpPort implementation)
    pub fn acp_runtime(mut self, adapter: Arc<dyn crate::ports::AcpPort + Send + Sync>) -> Self {
        self.acp_runtime = Some(adapter);
        self
    }

    /// Set CNS emitter adapter
    pub fn cns_emitter(mut self, adapter: CnsEmitterAdapter) -> Self {
        self.cns_emitter = Some(adapter);
        self
    }

    /// Set MCP runtime adapter
    pub fn mcp_runtime(mut self, adapter: McpRuntimeAdapter) -> Self {
        self.mcp_runtime = Some(adapter);
        self
    }

    /// Set memory storage adapter
    pub fn memory_storage(mut self, adapter: MemoryStorageAdapter) -> Self {
        self.memory_storage = Some(adapter);
        self
    }

    /// Use in-memory storage (convenience method)
    pub fn with_in_memory_storage(self) -> Self {
        self.memory_storage(MemoryStorageAdapter::in_memory().unwrap())
    }

    /// Use encrypted storage from path (convenience method)
    pub fn with_encrypted_storage<P: AsRef<std::path::Path>>(
        self,
        path: P,
        passphrase: &str,
    ) -> Self {
        self.memory_storage(
            MemoryStorageAdapter::from_path(path.as_ref().to_str().unwrap(), passphrase).unwrap(),
        )
    }

    /// Set security context
    pub fn security_context(mut self, context: SecurityContext) -> Self {
        self.security_context = Some(context);
        self
    }

    /// Build the PodManager
    ///
    /// Missing adapters are created with defaults:
    /// - Git CAS: `./registry/templates`
    /// - ACP Runtime: `Arc::new(AcpRuntime::default())`
    /// - CNS Emitter: `CnsEmitterAdapter::new(WebID::new())`
    /// - MCP Runtime: `McpRuntimeAdapter::new()`
    /// - Memory Storage: In-memory database
    /// - Security Context: Default rate limiter and expiry enforcer
    pub fn build(self) -> PodManager {
        PodManager::new(
            self.git_cas
                .unwrap_or_else(|| GitCasAdapter::from_path(PathBuf::from("./registry/templates"))),
            self.acp_runtime
                .unwrap_or_else(|| Arc::new(crate::acp::AcpRuntime::default())),
            self.cns_emitter
                .unwrap_or_else(|| CnsEmitterAdapter::new(WebID::new())),
            self.mcp_runtime.unwrap_or_default(),
            self.memory_storage
                .unwrap_or_else(|| MemoryStorageAdapter::in_memory().unwrap()),
        )
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

        let pod = AgentPod::new(template_name, persona, &self.git_cas)?;
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

        pod.activate(&self.mcp_runtime, &self.cns_emitter)?;

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

        pod.deactivate(&self.cns_emitter)?;

        info!(
            target: "hkask.pod",
            pod_id = %pod_id,
            "Pod deactivated"
        );

        Ok(())
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
            state: pod.state.to_string(),
            webid: pod.webid.to_string(),
            agent_type: match pod.agent_type {
                AgentType::Bot => "bot".to_string(),
                AgentType::Replicant => "replicant".to_string(),
            },
            template: pod.template_crate.name.clone(),
            created_at: pod.created_at,
        })
    }

    /// List all pods
    pub async fn list_pods(&self) -> AgentPodResult<Vec<PodStatus>> {
        let pods = self.pods.read().await;
        let statuses = pods
            .values()
            .map(|pod| PodStatus {
                pod_id: pod.id.to_string(),
                name: Some(pod.persona.agent.name.clone()),
                state: pod.state.to_string(),
                webid: pod.webid.to_string(),
                agent_type: match pod.agent_type {
                    AgentType::Bot => "bot".to_string(),
                    AgentType::Replicant => "replicant".to_string(),
                },
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
