use serde::{Deserialize, Serialize};

pub type UserProfile = hkask_storage::UserProfile;
pub type Contact = hkask_storage::Contact;
pub type ScheduledTask = hkask_storage::ScheduledTask;

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
    /// expect: "System types preserve semantic identity and are provenance-aware"
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
    /// expect: "System types preserve semantic identity and are provenance-aware"
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
