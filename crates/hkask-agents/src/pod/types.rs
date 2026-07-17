//! Pod value types — PodLifecycleState, PodID, persona types, template types

use hkask_capability::{CapabilitySpec, DelegationResource};
pub use hkask_types::AgentKind;
pub use hkask_types::PodID;
use hkask_types::WebID;
use serde::{Deserialize, Serialize};

use super::AgentPodError;

/// Agent operating mode — how the agent is currently interacting with the world.
///
/// Initially mutually exclusive: an agent can be in Chat mode OR Server mode,
/// not both. Concurrency support planned for future release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentMode {
    /// Conversational mode: chatting with users/agents, calling tools.
    Chat,
    /// Server mode: presenting as MCP server(s), handling incoming tool calls.
    Server,
}

impl std::fmt::Display for AgentMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentMode::Chat => write!(f, "chat"),
            AgentMode::Server => write!(f, "server"),
        }
    }
}

/// Pod tier — determines isolation model and filename convention.
///
/// - Curator: singleton, owns SemanticIndex, CNS aggregation
/// - Team: shared bot workspace, bots share episodic
/// - Replicant: per-user sovereign pod
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PodKind {
    Curator,
    Team,
    #[default]
    Replicant,
}

impl std::fmt::Display for PodKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PodKind::Curator => write!(f, "curator"),
            PodKind::Team => write!(f, "team"),
            PodKind::Replicant => write!(f, "replicant"),
        }
    }
}

/// Pod lifecycle state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PodLifecycleState {
    /// Pod instantiated from template crate, not yet registered
    Populated,
    /// Registered with A2A runtime, capability token minted
    Registered,
    /// Activated for A2A communication, MCP access granted
    Activated,
    /// Deactivated, capabilities revoked
    Deactivated,
}

impl PodLifecycleState {
    /// Whether a transition from `self` to `next` is legal.
    ///
    /// The lifecycle is a linear progression:
    /// `Populated → Registered → Activated → Deactivated`
    ///
    /// \[DECLARATIVE\] Re-stating the current state is a no-op and always permitted. (P7 — Evolutionary Architecture).
    /// Terminal state `Deactivated` admits no further transitions.
    ///
    /// expect: "Agent interactions are gated by OCAP boundaries"
    /// \[P4\] Motivating: Clear Boundaries — lifecycle state machine enforces transitions
    /// \[P7\] Constraining: Evolutionary Architecture — linear model + idempotent restate
    /// pre:  `self` and `next` are valid `PodLifecycleState` variants.
    /// post: Returns `true` if `self == next` (idempotent) or if the
    ///       transition follows the linear progression; `false` for all
    ///       other transitions (including from `Deactivated`).
    pub fn can_transition_to(&self, next: PodLifecycleState) -> bool {
        if *self == next {
            return true;
        }
        match (self, next) {
            (PodLifecycleState::Populated, PodLifecycleState::Registered)
            | (PodLifecycleState::Registered, PodLifecycleState::Activated)
            | (PodLifecycleState::Activated, PodLifecycleState::Deactivated) => true,
            // Deactivated is terminal; all other moves illegal.
            _ => false,
        }
    }
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

/// Agent persona definition (from YAML)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPersona {
    /// Agent identity
    pub(crate) agent: AgentIdentity,
    /// Agent charter (purpose and scope)
    pub(crate) charter: AgentCharter,
    /// Capabilities this agent requires
    pub capabilities: Vec<String>,
    /// Rights (access permissions)
    pub(crate) rights: Vec<AccessRight>,
    /// Responsibilities (obligations)
    pub responsibilities: Vec<String>,
    /// Default visibility for artifacts
    pub(crate) visibility: VisibilitySettings,
    /// Communication posture — when to speak, how to accommodate (CAT framework).
    /// None means the agent uses the default moderate-engagement posture.
    #[serde(default)]
    pub communication_posture: Option<CommunicationPosture>,
    /// Cached WebID (derived deterministically from persona)
    #[serde(skip, default)]
    cached_webid: Option<WebID>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AgentIdentity {
    pub name: String,
    #[serde(rename = "type")]
    pub agent_type: AgentKind,
    #[serde(default = "default_version")]
    pub version: String,
}

fn default_version() -> String {
    "0.1.0".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AgentCharter {
    pub description: String,
    pub editor: String,
}

// ── Communication Accommodation Theory (CAT) — convergence posture ─────────

/// Communication posture — governs whether and how an agent engages via Matrix.
///
/// Grounded in Communication Accommodation Theory (Giles): convergence is the
/// single dimension along which agents decide to speak or remain silent.
///
/// - High convergence (≥ 0.7): agent accommodates strongly — adopts the
///   interlocutor's style, vocabulary, and pace. Speaks readily.
/// - Moderate convergence (0.3–0.7): balanced accommodation. Speaks when
///   addressed or when the topic touches charter domain.
/// - Low convergence (< 0.3): divergent posture — maintains distance.
///   Speaks rarely, and only to direct explicit requests.
///
/// The `convergence_bias` IS the "speak or remain silent" decision.
/// The `invariant_traits` anchor consistency — no accommodation compromises
/// these core identity traits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationPosture {
    /// Convergence bias: 0.0 (silent, divergent) to 1.0 (fully convergent).
    /// Default: 0.5 — balanced, responds to direct engagement.
    #[serde(default = "default_convergence_bias")]
    pub convergence_bias: f64,

    /// Core traits never compromised by accommodation (consistency anchor).
    /// E.g., ["precise"] means the agent stays precise even when converging.
    #[serde(default)]
    pub invariant_traits: Vec<String>,
}

fn default_convergence_bias() -> f64 {
    0.5
}

impl Default for CommunicationPosture {
    fn default() -> Self {
        Self {
            convergence_bias: 0.5,
            invariant_traits: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AccessRight {
    pub read: Option<String>,
    pub write: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct VisibilitySettings {
    #[serde(default = "default_shared_visibility")]
    pub default: hkask_types::Visibility,
    #[serde(default = "default_private_visibility")]
    pub episodic_override: hkask_types::Visibility,
}

fn default_shared_visibility() -> hkask_types::Visibility {
    hkask_types::Visibility::Shared
}

fn default_private_visibility() -> hkask_types::Visibility {
    hkask_types::Visibility::Private
}

impl AgentPersona {
    /// Create a minimal persona for system pods (Curator, team infrastructure).
    /// Delegates to `from_yaml` for single construction path, single WebID derivation.
    ///
    /// expect: "The system provides bounded agent pod identity with capability-gated lifecycle management"
    /// post: returns AgentPersona
    pub fn system(name: &str, agent_type: AgentKind) -> Self {
        let agent = AgentIdentity {
            name: name.to_string(),
            agent_type,
            version: "0.1.0".to_string(),
        };
        let canonical = serde_json::to_string(&agent).unwrap_or_else(|e| {
            tracing::error!(
                target: "hkask.agent.identity",
                error = %e,
                name = %name,
                "AgentIdentity serialization failed — WebID will be derived from name only"
            );
            name.to_string()
        });
        Self {
            agent,
            charter: AgentCharter {
                description: format!("System pod: {}", name),
                editor: "system".to_string(),
            },
            capabilities: vec!["tool:execute".to_string()],
            rights: vec![],
            responsibilities: vec!["curate_and_aggregate".to_string()],
            visibility: VisibilitySettings {
                default: hkask_types::Visibility::Shared,
                episodic_override: hkask_types::Visibility::Private,
            },
            cached_webid: Some(WebID::from_persona(canonical.as_bytes())),
            communication_posture: None,
        }
    }

    /// Parse agent persona from YAML string
    ///
    /// expect: "The system provides bounded agent pod identity with capability-gated lifecycle management"
    /// post: returns Result<Self, AgentPodError>
    pub fn from_yaml(yaml: &str) -> Result<Self, AgentPodError> {
        let mut persona: Self = serde_yaml_neo::from_str(yaml)
            .map_err(|e| AgentPodError::PersonaParseError(e.to_string()))?;

        // Compute and cache WebID
        let canonical = serde_json::to_string(&persona.agent).unwrap_or_else(|e| {
            tracing::error!(
                target: "hkask.agent.identity",
                error = %e,
                name = %persona.agent.name,
                "AgentIdentity serialization failed in from_yaml — WebID will be derived from name only"
            );
            persona.agent.name.clone()
        });
        persona.cached_webid = Some(WebID::from_persona(canonical.as_bytes()));

        Ok(persona)
    }

    /// Get the agent's WebID (derived deterministically from persona)
    ///
    /// expect: "The system provides bounded agent pod identity with capability-gated lifecycle management"
    /// post: returns WebID
    pub fn webid(&self) -> WebID {
        self.cached_webid.unwrap_or_else(|| {
            let canonical = serde_json::to_string(&self.agent).unwrap_or_else(|e| {
                tracing::error!(
                    target: "hkask.agent.identity",
                    error = %e,
                    name = %self.agent.name,
                    "AgentIdentity serialization failed in webid() — falling back to name"
                );
                self.agent.name.clone()
            });
            WebID::from_persona(canonical.as_bytes())
        })
    }

    /// Public accessor for the agent's name.
    ///
    /// expect: "The system provides bounded agent pod identity with capability-gated lifecycle management"
    /// post: returns agent's name as &str
    pub fn name(&self) -> &str {
        &self.agent.name
    }

    /// Get capabilities as DelegationResource enums
    /// using the canonical [`CapabilitySpec`] parser.
    ///
    /// expect: "The system provides bounded agent pod identity with capability-gated lifecycle management"
    /// post: returns `Vec<DelegationResource>`
    pub fn capability_resources(&self) -> Vec<DelegationResource> {
        self.capabilities
            .iter()
            .filter_map(|cap| CapabilitySpec::parse(cap).ok().map(|s| s.resource))
            .collect()
    }

    /// Validate persona fields.
    ///
    /// expect: "The system provides bounded agent pod identity with capability-gated lifecycle management"
    /// post: returns Result<(), AgentPodError>
    pub fn validate_fields(
        name: &str,
        agent_type: &str,
        version: &str,
        description: &str,
        editor: &str,
        capabilities: &[String],
    ) -> Result<(), super::AgentPodError> {
        if name.is_empty() {
            return Err(super::AgentPodError::PersonaParseError(
                "name is required".to_string(),
            ));
        }
        if name.len() > 64 {
            return Err(super::AgentPodError::PersonaParseError(
                "name too long (max 64 chars)".to_string(),
            ));
        }
        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(super::AgentPodError::PersonaParseError(
                "name: invalid format".to_string(),
            ));
        }

        if !["bot", "replicant"].contains(&agent_type) {
            return Err(super::AgentPodError::PersonaParseError(
                "agent_type must be 'bot' or 'replicant'".to_string(),
            ));
        }

        if version.is_empty() || version.len() > 32 {
            return Err(super::AgentPodError::PersonaParseError(
                "version: invalid format".to_string(),
            ));
        }

        if description.len() > 1000 {
            return Err(super::AgentPodError::PersonaParseError(
                "description too long (max 1000 chars)".to_string(),
            ));
        }

        if editor.is_empty() || editor.len() > 256 {
            return Err(super::AgentPodError::PersonaParseError(
                "editor: invalid format".to_string(),
            ));
        }

        if capabilities.len() > 20 {
            return Err(super::AgentPodError::PersonaParseError(
                "too many capabilities (max 20)".to_string(),
            ));
        }
        for cap in capabilities {
            if cap.len() > 128 {
                return Err(super::AgentPodError::PersonaParseError(
                    "capability name too long (max 128 chars)".to_string(),
                ));
            }
        }

        Ok(())
    }
}
