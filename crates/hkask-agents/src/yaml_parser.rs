//! YAML deserializer — parse stored agent YAML back into rich AgentDefinition.
//!
//! The store holds `hkask_types::RegisteredAgent.source_yaml` (the original YAML).
//! This module parses it into the rich `AgentDefinition` from hkask-agents,
//! recovering fields like `persona`, `process_manifest`, `voice_description` etc.
//! that the base `hkask_types::AgentDefinition` doesn't carry.

use serde::Deserialize;

use crate::types::agent::definition::AgentDefinition;
use crate::yaml_types::RawYamlAgent;

/// Parse an agent definition from stored YAML.
///
/// Accepts both `agent:` and `bot:` top-level sections.
/// Returns `Err(String)` on parse failure or missing name/type.
pub fn parse_agent_from_yaml(yaml_source: &str) -> Result<AgentDefinition, String> {
    let raw: RawYamlAgent =
        serde_yaml_neo::from_str(yaml_source).map_err(|e| format!("YAML parse error: {e}"))?;
    raw.build_definition(
        || "No 'agent:' or 'bot:' section in YAML".to_string(),
        |agent_type| format!("Unknown agent type '{}'", agent_type),
    )
}
