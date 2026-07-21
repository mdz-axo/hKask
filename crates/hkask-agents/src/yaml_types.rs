//! Shared YAML agent types — used by both the registry loader and YAML parser.
//!
//! Extracted from the duplicated definitions in `registry_loader.rs` and `yaml_parser.rs`.
//! Both modules define identically:
//! - YamlAgentHeader, YamlCharter, YamlPersona, RawYamlAgent (structs)
//! - convert_rights, convert_responsibilities (methods)
//!
//! The `into_agent_definition` method differs between the two modules
//! (different error types and source-path handling), so it stays in each module.

use crate::types::agent::PersonaConstraints;
use crate::types::agent::{AgentDefinition, AgentKind, Charter};
use crate::types::agent::{Responsibility, Right};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct YamlAgentHeader {
    pub(crate) name: String,
    #[serde(rename = "type")]
    pub(crate) agent_type: String,
    #[serde(default)]
    pub(crate) voice_description: Option<String>,
    #[serde(default)]
    pub(crate) voice_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct YamlCharter {
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) archetype: String,
    #[serde(default)]
    pub(crate) visibility: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct YamlPersona {
    #[serde(default)]
    pub(crate) tone: String,
    #[serde(default)]
    pub(crate) verbosity: String,
    #[serde(default)]
    pub(crate) formatting: String,
    #[serde(default)]
    pub(crate) forbidden: Vec<String>,
    #[serde(default)]
    pub(crate) required: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawYamlAgent {
    #[serde(default)]
    pub(crate) agent: Option<YamlAgentHeader>,
    #[serde(default)]
    pub(crate) bot: Option<YamlAgentHeader>,
    #[serde(default)]
    pub(crate) charter: Option<YamlCharter>,
    #[serde(default)]
    pub(crate) capabilities: Vec<String>,
    #[serde(default)]
    pub(crate) rights: Vec<std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub(crate) responsibilities: Vec<std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub(crate) persona: Option<YamlPersona>,
    #[serde(default)]
    pub(crate) depends_on: Vec<String>,
    #[serde(default)]
    pub(crate) process_manifest: Option<String>,
}

impl RawYamlAgent {
    pub(crate) fn convert_rights(
        rights: Vec<std::collections::HashMap<String, String>>,
    ) -> Vec<Right> {
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

    pub(crate) fn convert_responsibilities(
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

    /// Build an AgentDefinition from this YAML agent, reusing the shared
    /// header/charter/persona parsing. Caller provides the error constructor
    /// for missing-agent and unknown-type errors so each module can use its
    /// own error type.
    pub(crate) fn build_definition<E>(
        self,
        missing_agent_err: impl FnOnce() -> E,
        unknown_type_err: impl FnOnce(String) -> E,
    ) -> Result<AgentDefinition, E> {
        let header = self.agent.or(self.bot).ok_or_else(missing_agent_err)?;
        Ok(AgentDefinition {
            name: header.name,
            charter: self.charter.map(|c| Charter {
                description: c.description,
                archetype: c.archetype,
                visibility: c.visibility,
            }),
            capabilities: self.capabilities,
            rights: Self::convert_rights(self.rights),
            responsibilities: Self::convert_responsibilities(self.responsibilities),
            persona: self.persona.map(|p| PersonaConstraints {
                tone: p.tone,
                verbosity: p.verbosity,
                formatting: p.formatting,
                forbidden: p.forbidden,
                required: p.required,
            }),
            depends_on: self.depends_on,
            process_manifest: self.process_manifest,
            voice_description: header.voice_description,
            voice_id: header.voice_id,
        })
    }
}
