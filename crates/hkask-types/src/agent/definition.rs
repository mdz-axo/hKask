//! Agent Definition types — Loop 5 (Curation): agent lifecycle and governance
//!
//! The Curator manages agent registration, evaluation, rights, and responsibilities.
//! These types define the full identity of an agent as specified in registry YAML.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
/// Loop: Curation
pub enum AgentKind {
    Bot,
    Replicant,
}

impl AgentKind {
    /// Get string representation of agent kind.
    ///
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentKind::Bot => "Bot",
            AgentKind::Replicant => "Replicant",
        }
    }

    /// Return the persona kind string for this agent kind.
    ///
    /// Maps `Bot` → `"bot"`, `Replicant` → `"replicant"`.
    /// Return the persona kind string.
    ///
    pub fn as_persona_kind(&self) -> &'static str {
        match self {
            AgentKind::Bot => "bot",
            AgentKind::Replicant => "replicant",
        }
    }

    /// Parse agent kind from string.
    ///
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "Bot" | "bot" => Some(AgentKind::Bot),
            "Replicant" | "replicant" => Some(AgentKind::Replicant),
            _ => None,
        }
    }
}

impl std::fmt::Display for AgentKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// Loop: Curation
pub struct Charter {
    pub description: String,
    #[serde(default)]
    pub archetype: String,
    #[serde(default)]
    pub visibility: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
/// Loop: Curation
pub struct PersonaConstraints {
    #[serde(default)]
    pub tone: String,
    #[serde(default)]
    pub verbosity: String,
    #[serde(default)]
    pub formatting: String,
    #[serde(default)]
    pub forbidden: Vec<String>,
    #[serde(default)]
    pub required: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Loop: Curation
pub struct AgentDefinition {
    pub name: String,
    pub agent_kind: AgentKind,
    #[serde(default)]
    pub charter: Option<Charter>,
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub rights: Vec<super::profile::Right>,
    #[serde(default)]
    pub responsibilities: Vec<super::profile::Responsibility>,
    #[serde(default)]
    pub persona: Option<PersonaConstraints>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub process_manifest: Option<String>,
    /// Voice description for the replicant's TTS voice.
    /// Natural language description (e.g., "warm female, British accent, professional").
    #[serde(default)]
    pub voice_description: Option<String>,
    /// Selected voice ID from the local TTS catalog (e.g., "en-us", "en-uk").
    #[serde(default)]
    pub voice_id: Option<String>,
}

impl AgentDefinition {
    /// Get flattened rights strings.
    ///
    pub fn rights_flat(&self) -> Vec<String> {
        self.rights.iter().map(|r| r.to_display_string()).collect()
    }

    /// Get flattened responsibilities strings.
    ///
    pub fn responsibilities_flat(&self) -> Vec<String> {
        self.responsibilities
            .iter()
            .map(|r| r.to_display_string())
            .collect()
    }

    /// Compose a system prompt from the agent definition.
    ///
    pub fn compose_system_prompt(&self) -> String {
        let mut prompt = String::new();

        prompt.push_str(&format!(
            "You are {}, a {} in the hKask system.\n\n",
            self.name, self.agent_kind
        ));

        if let Some(charter) = &self.charter {
            prompt.push_str(&format!("## Charter\n{}\n\n", charter.description));
        }

        if !self.responsibilities.is_empty() {
            prompt.push_str("## Responsibilities\n");
            for r in &self.responsibilities_flat() {
                prompt.push_str(&format!("- {}\n", r));
            }
            prompt.push('\n');
        }

        if !self.rights.is_empty() {
            prompt.push_str("## Rights\n");
            for r in &self.rights_flat() {
                prompt.push_str(&format!("- {}\n", r));
            }
            prompt.push('\n');
        }

        if let Some(persona) = &self.persona {
            prompt.push_str("## Voice\n");
            if !persona.tone.is_empty() {
                prompt.push_str(&format!("Tone: {}\n", persona.tone));
            }
            if !persona.verbosity.is_empty() {
                prompt.push_str(&format!("Verbosity: {}\n", persona.verbosity));
            }
            if !persona.forbidden.is_empty() {
                prompt.push_str(&format!("Never use: {}\n", persona.forbidden.join(", ")));
            }
            if !persona.required.is_empty() {
                prompt.push_str(&format!("Always: {}\n", persona.required.join(", ")));
            }
        }

        prompt
    }

    /// Check if the agent has a specific capability.
    ///
    pub fn has_capability(&self, cap: &str) -> bool {
        self.capabilities.iter().any(|c| c == cap)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Loop: Curation
pub struct RegisteredAgent {
    pub definition: AgentDefinition,
    pub token_hash: String,
    pub registered_at: String,
    pub source_yaml: String,
}
