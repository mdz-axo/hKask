//! Agent definition types — canonical definitions for agent kinds, profiles, and registrations

use serde::{Deserialize, Serialize};

/// Kind of agent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    Bot,
    Replicant,
}

impl AgentKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentKind::Bot => "bot",
            AgentKind::Replicant => "replicant",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "bot" => Some(AgentKind::Bot),
            "replicant" => Some(AgentKind::Replicant),
            _ => None,
        }
    }

    pub fn as_persona_kind(&self) -> &'static str {
        match self {
            AgentKind::Bot => "bot",
            AgentKind::Replicant => "replicant",
        }
    }
}

impl std::fmt::Display for AgentKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Charter — defines what an agent may do
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Charter {
    pub purpose: String,
    pub constraints: Vec<String>,
}

/// User profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
}

impl UserProfile {
    pub fn replicant_display_name(&self, chosen_first_name: &str) -> String {
        if chosen_first_name.is_empty() {
            format!("{} {}", self.first_name, self.last_name)
        } else {
            format!("{} ({})", chosen_first_name, self.last_name)
        }
    }
}

/// A right granted to an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Right {
    pub name: String,
    pub description: String,
}

/// A responsibility held by an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Responsibility {
    pub name: String,
    pub description: String,
}

/// Persona constraints for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaConstraints {
    pub tone: Option<String>,
    pub formality: Option<String>,
    pub verbosity: Option<String>,
}

/// Contact entry for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub agent_name: String,
    pub contact_name: String,
    #[serde(default)]
    pub relationship: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

/// Scheduled task for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub agent_name: String,
    pub trigger: String,
    pub action: String,
    #[serde(default)]
    pub params: Option<String>,
    #[serde(default)]
    pub next_run: Option<String>,
    #[serde(default)]
    pub enabled: bool,
}

/// Agent definition — core specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub name: String,
    pub agent_kind: AgentKind,
    #[serde(default)]
    pub charter: Option<Charter>,
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub rights: Vec<Right>,
    #[serde(default)]
    pub responsibilities: Vec<Responsibility>,
}

/// Registered agent — an agent definition plus registration metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredAgent {
    pub definition: AgentDefinition,
    pub token_hash: String,
    pub registered_at: String,
    pub source_yaml: String,
}
