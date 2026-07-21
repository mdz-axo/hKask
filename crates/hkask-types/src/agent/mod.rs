//! Agent definition types — canonical definitions for agent kinds, profiles, and registrations

use serde::{Deserialize, Serialize};

/// Persona constraints — tone, verbosity, and forbidden patterns.
///
/// Used by the persona filter in hkask-agents to enforce behavioral boundaries
/// on agent output. Define these in agent YAML; loaded at agent switch time.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
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
