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
//! use hkask_agents::adapters::acp_runtime::AcpRuntimeAdapter;
//! use hkask_agents::adapters::cns_emitter::CnsEmitterAdapter;
//! use hkask_agents::adapters::mcp_runtime::McpRuntimeAdapter;
//! use hkask_types::WebID;
//!
//! // Create adapters
//! let git_adapter = MockGitCas::new();
//! let acp_adapter = AcpRuntimeAdapter::new();
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
//! pod.register(&acp_adapter, &cns_emitter)?;
//! pod.activate(&mcp_runtime, &cns_emitter)?;
//! # Ok(())
//! # }
//! ```

use hkask_keystore::keychain::Keychain;
use hkask_types::{CapabilityAction, CapabilityResource, CapabilityToken, WebID};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::info;
use zeroize::Zeroizing;

use crate::adapters::git_cas::MockGitCas;

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
    pub fn register(
        &mut self,
        acp: &dyn ACPRuntimePort,
        cns: &dyn CNSSpanPort,
    ) -> AgentPodResult<()> {
        if self.state != PodLifecycleState::Populated {
            return Err(AgentPodError::InvalidStateTransition(
                self.state,
                PodLifecycleState::Registered,
            ));
        }

        let capabilities: Vec<String> = self.persona.capabilities.clone();
        let token = acp
            .register_agent(self.webid, capabilities)
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
    pub fn activate(
        &mut self,
        mcp: &dyn MCPRuntimePort,
        cns: &dyn CNSSpanPort,
    ) -> AgentPodResult<()> {
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
    pub fn deactivate(&mut self, cns: &dyn CNSSpanPort) -> AgentPodResult<()> {
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

/// ACP Runtime Port — Agent registration and identity management
pub trait ACPRuntimePort {
    /// Register an agent with the ACP runtime
    ///
    /// # Arguments
    /// * `webid` — Agent's WebID
    /// * `capabilities` — List of capability strings
    ///
    /// # Returns
    /// * `Ok(CapabilityToken)` — Registered capability token
    /// * `Err(String)` — Registration error message
    fn register_agent(
        &self,
        webid: WebID,
        capabilities: Vec<String>,
    ) -> Result<CapabilityToken, String>;
}

/// MCP Runtime Port — Tool access and invocation
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

/// CNS Span Port — Cybernetic event emission
pub trait CNSSpanPort {
    /// Emit a CNS span event
    ///
    /// # Arguments
    /// * `span` — Span name (e.g., "cns.agent_pod.registered")
    /// * `phase` — Event phase (e.g., "registered", "activated")
    /// * `observation` — Event observation as JSON
    /// * `confidence` — Confidence score (0.0 to 1.0)
    fn emit_event(&self, span: &str, phase: &str, observation: &serde_json::Value, confidence: f64);
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
    /// Create a new pod manager
    pub fn new() -> Self {
        Self {
            pods: Arc::new(RwLock::new(HashMap::new())),
            keystore: Keychain::default(),
        }
    }

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
        let git = MockGitCas;
        let pod = AgentPod::new(template_name, persona, &git)?;
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
        let mut pods = self.pods.write().await;
        let _pod = pods
            .get_mut(pod_id)
            .ok_or_else(|| AgentPodError::ACPRegistrationError("Pod not found".to_string()))?;

        // Placeholder - needs actual MCP runtime and CNS emitter
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
        let _pod = pods
            .get_mut(pod_id)
            .ok_or_else(|| AgentPodError::ACPRegistrationError("Pod not found".to_string()))?;

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
}

impl Default for PodManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    pub struct MockACPRuntime;
    impl ACPRuntimePort for MockACPRuntime {
        fn register_agent(
            &self,
            webid: WebID,
            _capabilities: Vec<String>,
        ) -> Result<CapabilityToken, String> {
            Ok(CapabilityToken::new(
                CapabilityResource::Tool,
                "*".to_string(),
                CapabilityAction::Execute,
                WebID::new(),
                webid,
                b"test-secret",
            ))
        }
    }

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
            Ok(json!({"result": "success"}))
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

    #[test]
    fn test_pod_lifecycle() {
        let persona_yaml = r#"
agent:
  name: "test-bot"
  type: "Bot"
  version: "0.1.0"
charter:
  description: "Test bot"
  editor: "curator"
capabilities:
  - "tool:inference:call"
rights: []
responsibilities: []
visibility:
  default: "public"
  episodic_override: "private"
"#;

        let persona = AgentPersona::from_yaml(persona_yaml).unwrap();
        let git = MockGitCAS;
        let mut pod = AgentPod::new("test-crate", &persona, &git).unwrap();

        assert_eq!(pod.state(), PodLifecycleState::Populated);

        let acp = MockACPRuntime;
        let cns = MockCNSSpan;
        pod.register(&acp, &cns).unwrap();
        assert_eq!(pod.state(), PodLifecycleState::Registered);

        let mcp = MockMCPRuntime;
        pod.activate(&mcp, &cns).unwrap();
        assert_eq!(pod.state(), PodLifecycleState::Activated);
        assert!(pod.is_active());

        pod.deactivate(&cns).unwrap();
        assert_eq!(pod.state(), PodLifecycleState::Deactivated);
        assert!(!pod.is_active());
    }

    #[test]
    fn test_invalid_state_transitions() {
        let persona_yaml = r#"
agent:
  name: "test-bot"
  type: "Bot"
charter:
  description: "Test"
  editor: "curator"
capabilities: []
rights: []
responsibilities: []
visibility:
  default: "public"
  episodic_override: "private"
"#;

        let persona = AgentPersona::from_yaml(persona_yaml).unwrap();
        let git = MockGitCAS;
        let mut pod = AgentPod::new("test-crate", &persona, &git).unwrap();

        let cns = MockCNSSpan;
        let result = pod.deactivate(&cns);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AgentPodError::InvalidStateTransition(_, _)
        ));
    }

    #[test]
    fn test_capability_attenuation() {
        let persona_yaml = r#"
agent:
  name: "test-bot"
  type: "Bot"
charter:
  description: "Test"
  editor: "curator"
capabilities: []
rights: []
responsibilities: []
visibility:
  default: "public"
  episodic_override: "private"
"#;

        let persona = AgentPersona::from_yaml(persona_yaml).unwrap();
        let git = MockGitCAS;
        let pod = AgentPod::new("test-crate", &persona, &git).unwrap();

        let new_holder = WebID::new();
        let attenuated = pod.delegate(new_holder, 1000).unwrap();

        assert_eq!(attenuated.attenuation_level, 1);
    }

    #[test]
    fn test_attenuation_limit_enforcement() {
        let persona_yaml = r#"
agent:
  name: "test-bot"
  type: "Bot"
charter:
  description: "Test"
  editor: "curator"
capabilities: []
rights: []
responsibilities: []
visibility:
  default: "public"
  episodic_override: "private"
"#;

        let persona = AgentPersona::from_yaml(persona_yaml).unwrap();
        let git = MockGitCAS;
        let mut pod = AgentPod::new("test-crate", &persona, &git).unwrap();

        let mut token = pod.capability_token.clone();
        token.attenuation_level = MAX_ATTENUATION_LEVEL;
        pod.capability_token = token;

        let new_holder = WebID::new();
        let result = pod.delegate(new_holder, 1000);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AgentPodError::AttenuationLimitExceeded
        ));
    }

    #[test]
    fn test_persona_parsing() {
        let yaml = r#"
agent:
  name: "memory-bot"
  type: "Bot"
  version: "0.2.0"
charter:
  description: "Expert bot for memory operations"
  editor: "curator"
capabilities:
  - "tool:memory:remember"
  - "tool:memory:recall"
rights:
  - read: "public_semantic_memory"
  - write: "own_episodic_memory"
responsibilities:
  - "respond_to: memory_tool_calls"
  - "emit: cns.agent_pod.*"
visibility:
  default: "public"
  episodic_override: "private"
"#;

        let persona = AgentPersona::from_yaml(yaml).unwrap();
        assert_eq!(persona.agent.name, "memory-bot");
        assert_eq!(persona.agent.agent_type, AgentType::Bot);
        assert_eq!(persona.agent.version, "0.2.0");
        assert_eq!(persona.capabilities.len(), 2);
    }

    #[test]
    fn test_double_registration_fails() {
        let persona_yaml = r#"
agent:
  name: "test-bot"
  type: "Bot"
charter:
  description: "Test"
  editor: "curator"
capabilities: []
rights: []
responsibilities: []
visibility:
  default: "public"
  episodic_override: "private"
"#;

        let persona = AgentPersona::from_yaml(persona_yaml).unwrap();
        let git = MockGitCAS;
        let mut pod = AgentPod::new("test-crate", &persona, &git).unwrap();

        let acp = MockACPRuntime;
        let cns = MockCNSSpan;
        pod.register(&acp, &cns).unwrap();

        let result = pod.register(&acp, &cns);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AgentPodError::InvalidStateTransition(_, _)
        ));
    }

    #[test]
    fn test_double_activation_fails() {
        let persona_yaml = r#"
agent:
  name: "test-bot"
  type: "Bot"
charter:
  description: "Test"
  editor: "curator"
capabilities: []
rights: []
responsibilities: []
visibility:
  default: "public"
  episodic_override: "private"
"#;

        let persona = AgentPersona::from_yaml(persona_yaml).unwrap();
        let git = MockGitCAS;
        let mut pod = AgentPod::new("test-crate", &persona, &git).unwrap();

        let acp = MockACPRuntime;
        let cns = MockCNSSpan;
        pod.register(&acp, &cns).unwrap();

        let mcp = MockMCPRuntime;
        pod.activate(&mcp, &cns).unwrap();

        let result = pod.activate(&mcp, &cns);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AgentPodError::InvalidStateTransition(_, _)
        ));
    }

    #[test]
    fn test_deactivate_from_populated_fails() {
        let persona_yaml = r#"
agent:
  name: "test-bot"
  type: "Bot"
charter:
  description: "Test"
  editor: "curator"
capabilities: []
rights: []
responsibilities: []
visibility:
  default: "public"
  episodic_override: "private"
"#;

        let persona = AgentPersona::from_yaml(persona_yaml).unwrap();
        let git = MockGitCAS;
        let mut pod = AgentPod::new("test-crate", &persona, &git).unwrap();

        let cns = MockCNSSpan;
        let result = pod.deactivate(&cns);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AgentPodError::InvalidStateTransition(_, _)
        ));
    }
}
