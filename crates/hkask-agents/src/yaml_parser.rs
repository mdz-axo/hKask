//! YAML deserializer — parse stored agent YAML back into rich AgentDefinition.
//!
//! The store holds `hkask_types::RegisteredAgent.source_yaml` (the original YAML).
//! This module parses it into the rich `AgentDefinition` from hkask-agents,
//! recovering fields like `persona`, `process_manifest`, `voice_description` etc.
//! that the base `hkask_types::AgentDefinition` doesn't carry.

use serde::Deserialize;

use crate::types::agent::definition::{AgentDefinition, Charter};
use crate::types::agent::profile::{Responsibility, Right};
use hkask_types::AgentKind;
use hkask_types::PersonaConstraints;

/// Parse an agent definition from stored YAML.
///
/// Accepts both `agent:` and `bot:` top-level sections.
/// Returns `Err(String)` on parse failure or missing name/type.
pub fn parse_agent_from_yaml(yaml_source: &str) -> Result<AgentDefinition, String> {
    let raw: RawYamlAgent =
        serde_yaml_neo::from_str(yaml_source).map_err(|e| format!("YAML parse error: {e}"))?;
    raw.into_agent_definition()
}

// ── Internal YAML structures ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct YamlAgentHeader {
    name: String,
    #[serde(rename = "type")]
    agent_type: String,
    #[serde(default)]
    voice_description: Option<String>,
    #[serde(default)]
    voice_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct YamlCharter {
    description: String,
    #[serde(default)]
    archetype: String,
    #[serde(default)]
    visibility: String,
}

#[derive(Debug, Deserialize)]
struct YamlPersona {
    #[serde(default)]
    tone: String,
    #[serde(default)]
    verbosity: String,
    #[serde(default)]
    formatting: String,
    #[serde(default)]
    forbidden: Vec<String>,
    #[serde(default)]
    required: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RawYamlAgent {
    #[serde(default)]
    agent: Option<YamlAgentHeader>,
    #[serde(default)]
    bot: Option<YamlAgentHeader>,
    #[serde(default)]
    charter: Option<YamlCharter>,
    #[serde(default)]
    capabilities: Vec<String>,
    #[serde(default)]
    rights: Vec<std::collections::HashMap<String, String>>,
    #[serde(default)]
    responsibilities: Vec<std::collections::HashMap<String, String>>,
    #[serde(default)]
    persona: Option<YamlPersona>,
    #[serde(default)]
    depends_on: Vec<String>,
    #[serde(default)]
    process_manifest: Option<String>,
}

impl RawYamlAgent {
    fn header(&self) -> Option<&YamlAgentHeader> {
        self.agent.as_ref().or(self.bot.as_ref())
    }

    fn convert_rights(rights: Vec<std::collections::HashMap<String, String>>) -> Vec<Right> {
        rights
            .into_iter()
            .filter_map(|map| {
                if let Some(resource) = map.get("read") {
                    Some(Right::Read {
                        resource: resource.clone(),
                    })
                } else if let Some(resource) = map.get("write") {
                    Some(Right::Write {
                        resource: resource.clone(),
                    })
                } else if let Some(action) = map.get("execute") {
                    Some(Right::Execute {
                        action: action.clone(),
                    })
                } else if let Some(scope) = map.get("coordinate") {
                    Some(Right::Coordinate {
                        scope: scope.clone(),
                    })
                } else {
                    map.get("escalate_to").map(|target| Right::EscalateTo {
                        target: target.clone(),
                    })
                }
            })
            .collect()
    }

    fn convert_responsibilities(
        responsibilities: Vec<std::collections::HashMap<String, String>>,
    ) -> Vec<Responsibility> {
        responsibilities
            .into_iter()
            .filter_map(|map| {
                if let Some(target) = map.get("monitor") {
                    Some(Responsibility::Monitor {
                        target: target.clone(),
                    })
                } else if let Some(input) = map.get("synthesize") {
                    Some(Responsibility::Synthesize {
                        input: input.clone(),
                        output: String::new(),
                    })
                } else if let Some(action) = map.get("perform") {
                    Some(Responsibility::Perform {
                        action: action.clone(),
                    })
                } else if let Some(target) = map.get("calibrate") {
                    Some(Responsibility::Calibrate {
                        target: target.clone(),
                    })
                } else if let Some(trigger) = map.get("escalate") {
                    Some(Responsibility::Escalate {
                        trigger: trigger.clone(),
                        target: String::new(),
                    })
                } else if let Some(resource) = map.get("maintain") {
                    Some(Responsibility::Maintain {
                        resource: resource.clone(),
                    })
                } else if let Some(span) = map.get("emit") {
                    Some(Responsibility::Emit { span: span.clone() })
                } else if let Some(session) = map.get("orchestrate") {
                    Some(Responsibility::Orchestrate {
                        session: session.clone(),
                    })
                } else if let Some(target) = map.get("record") {
                    Some(Responsibility::Record {
                        target: target.clone(),
                    })
                } else {
                    map.get("produce").map(|artifact| Responsibility::Produce {
                        artifact: artifact.clone(),
                    })
                }
            })
            .collect()
    }

    fn into_agent_definition(self) -> Result<AgentDefinition, String> {
        // Destructure self into locals to avoid borrow conflicts
        let RawYamlAgent {
            agent,
            bot,
            charter,
            capabilities,
            rights,
            responsibilities,
            persona,
            depends_on,
            process_manifest,
        } = self;

        let header = agent
            .or(bot)
            .ok_or_else(|| "No 'agent:' or 'bot:' section in YAML".to_string())?;
        let agent_kind = AgentKind::parse(&header.agent_type)
            .ok_or_else(|| format!("Unknown agent type '{}'", header.agent_type))?;

        Ok(AgentDefinition {
            name: header.name,
            agent_kind,
            charter: charter.map(|c| Charter {
                description: c.description,
                archetype: c.archetype,
                visibility: c.visibility,
            }),
            capabilities,
            rights: Self::convert_rights(rights),
            responsibilities: Self::convert_responsibilities(responsibilities),
            persona: persona.map(|p| PersonaConstraints {
                tone: p.tone,
                verbosity: p.verbosity,
                formatting: p.formatting,
                forbidden: p.forbidden,
                required: p.required,
            }),
            depends_on,
            process_manifest,
            voice_description: header.voice_description,
            voice_id: header.voice_id,
        })
    }
}
