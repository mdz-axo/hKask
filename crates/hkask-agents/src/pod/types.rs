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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persona_webid_deterministic() {
        let yaml = r#"
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
    }
}
