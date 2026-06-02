//! Agent Definition types — Loop 5 (Curation): agent lifecycle and governance
//!
//! The Curator manages agent registration, evaluation, rights, and responsibilities.
//! These types define the full identity of an agent as specified in registry YAML.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Loop: Curation
/// Access right granted to an agent (R9: Structured Data Modeling)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Right {
    Read { resource: String },
    Write { resource: String },
    Execute { action: String },
    Coordinate { scope: String },
    EscalateTo { target: String },
}

impl Right {
    pub fn to_display_string(&self) -> String {
        match self {
            Right::Read { resource } => format!("read: {}", resource),
            Right::Write { resource } => format!("write: {}", resource),
            Right::Execute { action } => format!("execute: {}", action),
            Right::Coordinate { scope } => format!("coordinate: {}", scope),
            Right::EscalateTo { target } => format!("escalate_to: {}", target),
        }
    }
}

/// Loop: Curation
/// Responsibility assigned to an agent (R9: Structured Data Modeling)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Responsibility {
    Monitor { target: String },
    Synthesize { input: String, output: String },
    Perform { action: String },
    Calibrate { target: String },
    Escalate { trigger: String, target: String },
    Maintain { resource: String },
    Emit { span: String },
    Orchestrate { session: String },
    Record { target: String },
    Produce { artifact: String },
}

impl Responsibility {
    pub fn to_display_string(&self) -> String {
        match self {
            Responsibility::Monitor { target } => format!("monitor: {}", target),
            Responsibility::Synthesize { input, output } => {
                format!("synthesize: {} -> {}", input, output)
            }
            Responsibility::Perform { action } => format!("perform: {}", action),
            Responsibility::Calibrate { target } => format!("calibrate: {}", target),
            Responsibility::Escalate { trigger, target } => {
                format!("escalate: {} -> {}", trigger, target)
            }
            Responsibility::Maintain { resource } => format!("maintain: {}", resource),
            Responsibility::Emit { span } => format!("emit: {}", span),
            Responsibility::Orchestrate { session } => format!("orchestrate: {}", session),
            Responsibility::Record { target } => format!("record: {}", target),
            Responsibility::Produce { artifact } => format!("produce: {}", artifact),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
/// Loop: Curation
pub enum AgentKind {
    Bot,
    Replicant,
}

impl AgentKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentKind::Bot => "Bot",
            AgentKind::Replicant => "Replicant",
        }
    }

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// Loop: Curation
pub struct ReadinessProbe {
    #[serde(rename = "type")]
    pub probe_type: String,
    pub endpoint: String,
    #[serde(default)]
    pub expected: HashMap<String, serde_json::Value>,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    #[serde(default = "default_retry")]
    pub retry_count: u32,
}

fn default_timeout() -> u64 {
    15
}

fn default_retry() -> u32 {
    3
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// Loop: Curation
/// Per-agent standing session configuration (part of agent definition)
pub struct AgentStandingSessionConfig {
    pub session_id: String,
    pub role: String,
    #[serde(default)]
    pub report_interval: String,
    #[serde(default)]
    pub administrator_visible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// Loop: Curation
pub struct ReportingConfig {
    #[serde(default)]
    pub escalate_to: Option<String>,
    #[serde(default)]
    pub report_to: Option<String>,
    #[serde(default)]
    pub report_format: Option<String>,
    #[serde(default)]
    pub alert_threshold: Option<String>,
    #[serde(default)]
    pub report_interval: Option<String>,
    #[serde(default)]
    pub report_on: Vec<String>,
    #[serde(default)]
    pub receives_from: Vec<String>,
    #[serde(default)]
    pub escalation_triggers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Loop: Curation
pub struct AgentDefinition {
    pub name: String,
    pub agent_kind: AgentKind,
    #[serde(default)]
    pub binding_contract: bool,
    #[serde(default)]
    pub editor: String,
    #[serde(default)]
    pub charter: Option<Charter>,
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub rights: Vec<Right>,
    #[serde(default)]
    pub responsibilities: Vec<Responsibility>,
    #[serde(default)]
    pub reporting: Option<ReportingConfig>,
    #[serde(default)]
    pub standing_session: Option<AgentStandingSessionConfig>,
    #[serde(default)]
    pub persona: Option<PersonaConstraints>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub readiness_probe: Option<ReadinessProbe>,
    #[serde(default)]
    pub process_manifest: Option<String>,
}

impl AgentDefinition {
    pub fn rights_flat(&self) -> Vec<String> {
        self.rights.iter().map(|r| r.to_display_string()).collect()
    }

    pub fn responsibilities_flat(&self) -> Vec<String> {
        self.responsibilities
            .iter()
            .map(|r| r.to_display_string())
            .collect()
    }

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
