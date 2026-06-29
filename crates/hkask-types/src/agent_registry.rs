//! Agent registry types — canonical agent definition schema.
//!
//! This module defines the unified, YAML-facing agent schema used by the
//! registry loader, storage layer, and agent runtime. It is the canonical
//! representation for agent definitions, rights, responsibilities, and
//! related registry records.

use crate::agent::{AgentKind, PersonaConstraints};
use serde::{Deserialize, Serialize};

/// Charter — defines what an agent may do.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Charter {
    /// Primary charter description.
    #[serde(alias = "purpose")]
    pub description: String,
    /// Optional archetype label (e.g., "curator", "builder").
    #[serde(default)]
    pub archetype: String,
    /// Default visibility label (e.g., "public", "private").
    #[serde(default)]
    pub visibility: String,
    /// Legacy constraints list (storage compatibility).
    #[serde(default)]
    pub constraints: Vec<String>,
}

/// Access right granted to an agent.
///
/// Serialized as a tagged enum (type + fields). Legacy flat records are
/// accepted via custom deserialization and mapped to `Legacy`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Right {
    Read {
        resource: String,
    },
    Write {
        resource: String,
    },
    Execute {
        action: String,
    },
    Coordinate {
        scope: String,
    },
    EscalateTo {
        target: String,
    },
    /// Legacy flat record (name + description).
    Legacy {
        name: String,
        description: String,
    },
}

impl Right {
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is a valid Right variant
    /// post: returns a human-readable display string
    pub fn to_display_string(&self) -> String {
        match self {
            Right::Read { resource } => format!("read: {resource}"),
            Right::Write { resource } => format!("write: {resource}"),
            Right::Execute { action } => format!("execute: {action}"),
            Right::Coordinate { scope } => format!("coordinate: {scope}"),
            Right::EscalateTo { target } => format!("escalate_to: {target}"),
            Right::Legacy { name, description } => format!("{name}: {description}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum RightTagged {
    Read { resource: String },
    Write { resource: String },
    Execute { action: String },
    Coordinate { scope: String },
    EscalateTo { target: String },
}

impl From<RightTagged> for Right {
    fn from(tagged: RightTagged) -> Self {
        match tagged {
            RightTagged::Read { resource } => Right::Read { resource },
            RightTagged::Write { resource } => Right::Write { resource },
            RightTagged::Execute { action } => Right::Execute { action },
            RightTagged::Coordinate { scope } => Right::Coordinate { scope },
            RightTagged::EscalateTo { target } => Right::EscalateTo { target },
        }
    }
}

impl<'de> Deserialize<'de> for Right {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        if value.get("type").is_some() {
            let tagged: RightTagged =
                serde_json::from_value(value).map_err(serde::de::Error::custom)?;
            return Ok(tagged.into());
        }
        let name = value
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let description = value
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        Ok(Right::Legacy {
            name: name.to_string(),
            description: description.to_string(),
        })
    }
}

/// Responsibility assigned to an agent.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Responsibility {
    Monitor {
        target: String,
    },
    Synthesize {
        input: String,
        output: String,
    },
    Perform {
        action: String,
    },
    Calibrate {
        target: String,
    },
    Escalate {
        trigger: String,
        target: String,
    },
    Maintain {
        resource: String,
    },
    Emit {
        span: String,
    },
    Orchestrate {
        session: String,
    },
    Record {
        target: String,
    },
    Produce {
        artifact: String,
    },
    /// Legacy flat record (name + description).
    Legacy {
        name: String,
        description: String,
    },
}

impl Responsibility {
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is a valid Responsibility variant
    /// post: returns a human-readable display string
    pub fn to_display_string(&self) -> String {
        match self {
            Responsibility::Monitor { target } => format!("monitor: {target}"),
            Responsibility::Synthesize { input, output } => {
                format!("synthesize: {input} -> {output}")
            }
            Responsibility::Perform { action } => format!("perform: {action}"),
            Responsibility::Calibrate { target } => format!("calibrate: {target}"),
            Responsibility::Escalate { trigger, target } => {
                format!("escalate: {trigger} -> {target}")
            }
            Responsibility::Maintain { resource } => format!("maintain: {resource}"),
            Responsibility::Emit { span } => format!("emit: {span}"),
            Responsibility::Orchestrate { session } => format!("orchestrate: {session}"),
            Responsibility::Record { target } => format!("record: {target}"),
            Responsibility::Produce { artifact } => format!("produce: {artifact}"),
            Responsibility::Legacy { name, description } => format!("{name}: {description}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ResponsibilityTagged {
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

impl From<ResponsibilityTagged> for Responsibility {
    fn from(tagged: ResponsibilityTagged) -> Self {
        match tagged {
            ResponsibilityTagged::Monitor { target } => Responsibility::Monitor { target },
            ResponsibilityTagged::Synthesize { input, output } => {
                Responsibility::Synthesize { input, output }
            }
            ResponsibilityTagged::Perform { action } => Responsibility::Perform { action },
            ResponsibilityTagged::Calibrate { target } => Responsibility::Calibrate { target },
            ResponsibilityTagged::Escalate { trigger, target } => {
                Responsibility::Escalate { trigger, target }
            }
            ResponsibilityTagged::Maintain { resource } => Responsibility::Maintain { resource },
            ResponsibilityTagged::Emit { span } => Responsibility::Emit { span },
            ResponsibilityTagged::Orchestrate { session } => {
                Responsibility::Orchestrate { session }
            }
            ResponsibilityTagged::Record { target } => Responsibility::Record { target },
            ResponsibilityTagged::Produce { artifact } => Responsibility::Produce { artifact },
        }
    }
}

impl<'de> Deserialize<'de> for Responsibility {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        if value.get("type").is_some() {
            let tagged: ResponsibilityTagged =
                serde_json::from_value(value).map_err(serde::de::Error::custom)?;
            return Ok(tagged.into());
        }
        let name = value
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let description = value
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        Ok(Responsibility::Legacy {
            name: name.to_string(),
            description: description.to_string(),
        })
    }
}

/// Agent definition — core specification as stored in the registry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    #[serde(default)]
    pub persona: Option<PersonaConstraints>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub process_manifest: Option<String>,
    /// Voice description for the replicant's TTS voice.
    #[serde(default)]
    pub voice_description: Option<String>,
    /// Selected voice ID from the local TTS catalog.
    #[serde(default)]
    pub voice_id: Option<String>,
}

impl AgentDefinition {
    /// Get flattened rights strings.
    pub fn rights_flat(&self) -> Vec<String> {
        self.rights.iter().map(|r| r.to_display_string()).collect()
    }

    /// Get flattened responsibilities strings.
    pub fn responsibilities_flat(&self) -> Vec<String> {
        self.responsibilities
            .iter()
            .map(|r| r.to_display_string())
            .collect()
    }

    /// Compose a system prompt from the agent definition.
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
    pub fn has_capability(&self, cap: &str) -> bool {
        self.capabilities.iter().any(|c| c == cap)
    }
}

/// Registered agent — an agent definition plus registration metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RegisteredAgent {
    pub definition: AgentDefinition,
    pub token_hash: String,
    pub registered_at: String,
    pub source_yaml: String,
}

/// The human user's identity — collected once during onboarding.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserProfile {
    pub first_name: String,
    pub last_name: String,
    /// Email address — forward-looking, no email MCP server yet.
    pub email: String,
}

impl UserProfile {
    /// Compose a replicant's full display name.
    pub fn replicant_display_name(&self, chosen_first_name: &str) -> String {
        if chosen_first_name.is_empty() {
            format!("{} {}", self.first_name, self.last_name)
        } else {
            format!("{} ({})", chosen_first_name, self.last_name)
        }
    }
}

/// A contact in an agent's personal contact registry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Contact {
    /// Which replicant owns this contact.
    pub agent_name: String,
    /// Display name for the contact.
    pub contact_name: String,
    /// Relationship to the human user (e.g., "lawyer", "partner", "client").
    #[serde(default)]
    pub relationship: Option<String>,
    /// Free-text notes.
    #[serde(default)]
    pub notes: Option<String>,
}

/// A scheduled task owned by an agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScheduledTask {
    /// Which replicant owns this task.
    pub agent_name: String,
    /// Cron-like trigger expression (e.g., "daily 7am", "0 9 * * mon-fri").
    pub trigger: String,
    /// Action to perform (e.g., "notify_user", "run_research").
    pub action: String,
    /// JSON parameters for the action.
    #[serde(default)]
    pub params: Option<String>,
    /// Next scheduled run time (ISO 8601), if scheduled.
    #[serde(default)]
    pub next_run: Option<String>,
    /// Whether this task is active.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}
