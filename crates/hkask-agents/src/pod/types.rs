//! Pod value types — PodLifecycleState, PodID, persona types, template types

use hkask_types::{CapabilityResource, WebID};
use serde::{Deserialize, Serialize};

// Import macro for PodID generation
use hkask_types::define_id_type;

pub use hkask_types::AgentKind;

use super::AgentPodError;

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

define_id_type!(PodID);

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
    pub agent_type: AgentKind,
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

    /// Validate persona fields directly, replacing the need for `AgentPersonaInput`.
    ///
    /// Validates name, agent_type, version, description, editor, and capabilities
    /// against the same rules previously enforced by `AgentPersonaInput::validate()`.
    pub fn validate_fields(
        name: &str,
        agent_type: &str,
        version: &str,
        description: &str,
        editor: &str,
        capabilities: &[String],
    ) -> Result<(), crate::security::ValidationError> {
        use crate::security::ValidationError;

        if name.is_empty() {
            return Err(ValidationError::MissingField("name".to_string()));
        }
        if name.len() > 64 {
            return Err(ValidationError::FieldTooLong {
                field: "name".to_string(),
                max: 64,
            });
        }
        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(ValidationError::InvalidFormat {
                field: "name".to_string(),
            });
        }

        if !["bot", "replicant"].contains(&agent_type) {
            return Err(ValidationError::InvalidFormat {
                field: "agent_type".to_string(),
            });
        }

        if version.is_empty() || version.len() > 32 {
            return Err(ValidationError::InvalidFormat {
                field: "version".to_string(),
            });
        }

        if description.len() > 1000 {
            return Err(ValidationError::FieldTooLong {
                field: "description".to_string(),
                max: 1000,
            });
        }

        if editor.is_empty() || editor.len() > 256 {
            return Err(ValidationError::InvalidFormat {
                field: "editor".to_string(),
            });
        }

        if capabilities.len() > 20 {
            return Err(ValidationError::InvalidFormat {
                field: "capabilities".to_string(),
            });
        }
        for cap in capabilities {
            if cap.len() > 128 {
                return Err(ValidationError::FieldTooLong {
                    field: "capability".to_string(),
                    max: 128,
                });
            }
        }

        Ok(())
    }
}

/// **Deprecated:** Use `AgentPersona` directly instead.
///
/// `AgentPersonaInput` has been collapsed into `AgentPersona`.
/// Use `AgentPersona::validate_fields()` for validation and construct
/// `AgentPersona` directly from its fields.
#[allow(deprecated)]
impl From<crate::security::AgentPersonaInput> for AgentPersona {
    fn from(input: crate::security::AgentPersonaInput) -> Self {
        let agent_type = match input.agent_type.as_str() {
            "bot" => AgentKind::Bot,
            _ => AgentKind::Replicant,
        };
        let agent = AgentIdentity {
            name: input.name,
            agent_type,
            version: input.version,
        };
        let charter = AgentCharter {
            description: input.description,
            editor: input.editor,
        };
        let visibility = VisibilitySettings {
            default: hkask_types::Visibility::Public,
            episodic_override: hkask_types::Visibility::Private,
        };
        Self::new(
            agent,
            charter,
            input.capabilities,
            vec![], // rights — not in AgentPersonaInput
            vec![], // responsibilities — not in AgentPersonaInput
            visibility,
        )
    }
}

/// Template crate structure (loaded from Git CAS)
///
/// Re-exported from `hkask_types::TemplateCrate` for backward compatibility.
/// The canonical definition lives in `hkask_types`.
pub use hkask_types::{TemplateCrate, TemplateFile};
