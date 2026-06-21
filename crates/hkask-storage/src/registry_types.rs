//! Agent registry storage types — simple structs for DB serialization.
//!
//! These were moved from `hkask_types` to resolve the circular dependency
//! between hkask-storage (can't depend on hkask-agents) and hkask-agents
//! (depends on hkask-storage).

pub use hkask_types::AgentKind;
use serde::{Deserialize, Serialize};

/// Charter — defines what an agent may do (storage format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Charter {
    pub purpose: String,
    pub constraints: Vec<String>,
}

/// A right granted to an agent (storage format — flat key-value)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Right {
    pub name: String,
    pub description: String,
}

/// A responsibility held by an agent (storage format — flat key-value)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Responsibility {
    pub name: String,
    pub description: String,
}

/// Agent definition — core specification (storage format)
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
