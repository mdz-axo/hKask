//! YAML deserializer — parse stored agent YAML back into AgentDefinition.
//!
//! The store holds `hkask_types::RegisteredAgent.source_yaml` (the original YAML).
//! This module parses it into the canonical `AgentDefinition`, rebuilding the
//! same fields (persona, process_manifest, voice_description, etc.) from source
//! YAML when needed.

use crate::types::agent::AgentDefinition;
use crate::yaml_types::RawYamlAgent;

/// Errors that can occur during YAML agent parsing.
#[derive(Debug, thiserror::Error)]
pub enum YamlParseError {
    #[error("YAML deserialization failed: {0}")]
    Deserialization(#[source] serde_yaml_neo::Error),
    #[error("missing agent or bot section in YAML")]
    MissingAgentSection,
    #[error("unknown agent type: {0}")]
    UnknownAgentType(String),
}

/// Parse an agent definition from stored YAML.
///
/// Accepts both `agent:` and `bot:` top-level sections.
pub fn parse_agent_from_yaml(yaml_source: &str) -> Result<AgentDefinition, YamlParseError> {
    let raw: RawYamlAgent =
        serde_yaml_neo::from_str(yaml_source).map_err(YamlParseError::Deserialization)?;
    raw.build_definition(
        || YamlParseError::MissingAgentSection,
        YamlParseError::UnknownAgentType,
    )
}
