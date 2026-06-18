use serde::{Deserialize, Serialize};

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
/// expect: "System types preserve semantic identity and are provenance-aware" [P8]
    /// pre:  self is a valid Right variant (Read, Write, Execute, Coordinate, or EscalateTo)
    /// post: returns a human-readable display string like "read: resource_name"
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
/// expect: "System types preserve semantic identity and are provenance-aware" [P8]
    /// pre:  self is a valid Responsibility variant
    /// post: returns a human-readable display string describing the
    ///       responsibility (e.g., "monitor: target", "synthesize: input -> output")
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

/// The human user's identity — collected once during first onboarding,
/// shared across all their replicants. Stored in the registry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserProfile {
    pub first_name: String,
    pub last_name: String,
    /// Email address — forward-looking, no email MCP server yet
    pub email: String,
}

impl UserProfile {
    /// Compose a replicant's full display name from the user-chosen first name
    /// and the human's last name, following the naming protocol:
    /// "{chosen_name} r{human_last_name}"
    ///
/// expect: "System types preserve semantic identity and are provenance-aware" [P8]
    /// pre:  chosen_first_name is a non-empty string; self.last_name is non-empty
    /// post: returns "{chosen_first_name} r{self.last_name}"
    pub fn replicant_display_name(&self, chosen_first_name: &str) -> String {
        format!("{} r{}", chosen_first_name, self.last_name)
    }
}

/// A contact in an agent's personal contact registry.
/// Each agent owns its own contacts, stored in the registry DB.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Contact {
    /// Which replicant owns this contact
    pub agent_name: String,
    /// Display name for the contact
    pub contact_name: String,
    /// Relationship to the human user (e.g., "lawyer", "partner", "client")
    #[serde(default)]
    pub relationship: Option<String>,
    /// Free-text notes
    #[serde(default)]
    pub notes: Option<String>,
}

/// A scheduled task owned by an agent. The curation loop checks for due
/// tasks each cycle and fires the associated action.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScheduledTask {
    /// Which replicant owns this task
    pub agent_name: String,
    /// Cron-like trigger expression (e.g., "daily 7am", "0 9 * * mon-fri")
    pub trigger: String,
    /// Action to perform (e.g., "notify_user", "run_research")
    pub action: String,
    /// JSON parameters for the action
    #[serde(default)]
    pub params: Option<String>,
    /// Next scheduled run time (ISO 8601)
    pub next_run: String,
    /// Whether this task is active
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}
