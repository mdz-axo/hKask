//! Agent definition types — canonical definitions for agent kinds, profiles, and registrations

use serde::{Deserialize, Serialize};

/// Kind of agent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    #[serde(alias = "Bot", alias = "BOT")]
    Bot,
    #[serde(alias = "Replicant", alias = "REPLICANT")]
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
        match s.to_lowercase().as_str() {
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
