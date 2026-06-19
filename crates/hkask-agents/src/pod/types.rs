//! Pod value types — PodLifecycleState, PodID, persona types, template types

pub use hkask_types::PodID;
use hkask_types::{CapabilitySpec, DelegationResource, WebID};
use serde::{Deserialize, Serialize};

pub use hkask_types::AgentKind;

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
    /// \[P4\] Motivating: Clear Boundaries — lifecycle state machine enforces transitions
    /// \[P7\] Constraining: Evolutionary Architecture — linear model + idempotent restate
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
    /// Cached WebID (derived deterministically from persona)
    #[serde(skip)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AccessRight {
    pub read: Option<String>,
    pub write: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct VisibilitySettings {
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
    /// Parse agent persona from YAML string
    pub fn from_yaml(yaml: &str) -> Result<Self, AgentPodError> {
        let mut persona: Self = serde_yaml_neo::from_str(yaml)
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

    /// Get capabilities as DelegationResource enums
    /// using the canonical [`CapabilitySpec`] parser.
    pub fn capability_resources(&self) -> Vec<DelegationResource> {
        self.capabilities
            .iter()
            .filter_map(|cap| CapabilitySpec::parse(cap).ok().map(|s| s.resource))
            .collect()
    }

    /// Validate persona fields.
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
