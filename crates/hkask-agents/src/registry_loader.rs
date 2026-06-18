//! AgentRegistryLoader — Load agent YAML definitions, register with A2A, persist to storage

use hkask_rsolidity as rs;
use crate::a2a::{A2AError, A2ARuntime};
use crate::ports::RegistrySourcePort;
use hkask_storage::{AgentRegistryError, AgentRegistryStore, now_rfc3339};
use hkask_types::{AgentDefinition, AgentKind, RegisteredAgent, WebID};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tracing::{info, warn};

#[derive(Error, Debug)]
pub enum RegistryLoaderError {
    #[error("IO error: {0}")]
    Io(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("YAML parse error in {path}: {source}")]
    YamlParse {
        path: String,
        source: serde_yaml_neo::Error,
    },
    #[error("A2A error: {0}")]
    A2A(#[from] A2AError),
    #[error("Storage error: {0}")]
    Storage(#[from] AgentRegistryError),
    #[error("Invalid agent definition: {0}")]
    InvalidDefinition(String),
}

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

    fn convert_rights(
        rights: Vec<std::collections::HashMap<String, String>>,
    ) -> Vec<hkask_types::Right> {
        rights
            .into_iter()
            .filter_map(|map| {
                if let Some(resource) = map.get("read") {
                    Some(hkask_types::Right::Read {
                        resource: resource.clone(),
                    })
                } else if let Some(resource) = map.get("write") {
                    Some(hkask_types::Right::Write {
                        resource: resource.clone(),
                    })
                } else if let Some(action) = map.get("execute") {
                    Some(hkask_types::Right::Execute {
                        action: action.clone(),
                    })
                } else if let Some(scope) = map.get("coordinate") {
                    Some(hkask_types::Right::Coordinate {
                        scope: scope.clone(),
                    })
                } else {
                    map.get("escalate_to")
                        .map(|target| hkask_types::Right::EscalateTo {
                            target: target.clone(),
                        })
                }
            })
            .collect()
    }

    fn convert_responsibilities(
        responsibilities: Vec<std::collections::HashMap<String, String>>,
    ) -> Vec<hkask_types::Responsibility> {
        responsibilities
            .into_iter()
            .filter_map(|map| {
                if let Some(target) = map.get("monitor") {
                    Some(hkask_types::Responsibility::Monitor {
                        target: target.clone(),
                    })
                } else if let Some(input) = map.get("synthesize") {
                    Some(hkask_types::Responsibility::Synthesize {
                        input: input.clone(),
                        output: String::new(),
                    })
                } else if let Some(action) = map.get("perform") {
                    Some(hkask_types::Responsibility::Perform {
                        action: action.clone(),
                    })
                } else if let Some(target) = map.get("calibrate") {
                    Some(hkask_types::Responsibility::Calibrate {
                        target: target.clone(),
                    })
                } else if let Some(trigger) = map.get("escalate") {
                    Some(hkask_types::Responsibility::Escalate {
                        trigger: trigger.clone(),
                        target: String::new(),
                    })
                } else if let Some(resource) = map.get("maintain") {
                    Some(hkask_types::Responsibility::Maintain {
                        resource: resource.clone(),
                    })
                } else if let Some(span) = map.get("emit") {
                    Some(hkask_types::Responsibility::Emit { span: span.clone() })
                } else if let Some(session) = map.get("orchestrate") {
                    Some(hkask_types::Responsibility::Orchestrate {
                        session: session.clone(),
                    })
                } else if let Some(target) = map.get("record") {
                    Some(hkask_types::Responsibility::Record {
                        target: target.clone(),
                    })
                } else {
                    map.get("produce")
                        .map(|artifact| hkask_types::Responsibility::Produce {
                            artifact: artifact.clone(),
                        })
                }
            })
            .collect()
    }

    fn into_agent_definition(
        self,
        source_path: &str,
    ) -> Result<AgentDefinition, RegistryLoaderError> {
        let header = self.header().ok_or_else(|| {
            RegistryLoaderError::InvalidDefinition(format!(
                "No 'agent:' or 'bot:' section in {}",
                source_path
            ))
        })?;
        let agent_kind = AgentKind::parse(&header.agent_type).ok_or_else(|| {
            RegistryLoaderError::InvalidDefinition(format!(
                "Unknown agent type '{}' in {}",
                header.agent_type, source_path
            ))
        })?;

        // Extract header fields before moving out of self (borrow checker)
        let _voice_description = header.voice_description.clone();
        let _voice_id = header.voice_id.clone();
        let header_name = header.name.clone();

        Ok(AgentDefinition {
            name: header_name,
            agent_kind,
            charter: self.charter.map(|c| hkask_types::Charter {
                description: c.description,
                archetype: c.archetype,
                visibility: c.visibility,
            }),
            capabilities: self.capabilities,
            rights: Self::convert_rights(self.rights),
            responsibilities: Self::convert_responsibilities(self.responsibilities),
            persona: self.persona.map(|p| hkask_types::PersonaConstraints {
                tone: p.tone,
                verbosity: p.verbosity,
                formatting: p.formatting,
                forbidden: p.forbidden,
                required: p.required,
            }),
            depends_on: self.depends_on,
            process_manifest: self.process_manifest,
            voice_description: _voice_description,
            voice_id: _voice_id,
        })
    }
}

pub struct AgentRegistryLoader {
    registry_path: PathBuf,
    a2a_runtime: Arc<A2ARuntime>,
    store: AgentRegistryStore,
    source: Arc<dyn RegistrySourcePort>,
}

impl AgentRegistryLoader {
    /// expect: "The system loads and adapts agent registries for generative use" [P3]
    /// \[P3\] Motivating: Generative Space — loader reads YAML agent definitions into registry
    /// pre:  `registry_path` is a valid `PathBuf`; `a2a_runtime` is a
    ///       valid `Arc<A2ARuntime>`; `store` is a valid
    ///       `AgentRegistryStore`; `source` is a valid
    ///       `Arc<dyn RegistrySourcePort>`.
    /// post: Returns an `AgentRegistryLoader` with all fields set.
    #[rs::contract(id = "P3-agt-registry-loader-new", principle = "P3")]
    #[rs::contract(id = "P3-agt-registry-loader-new", principle = "P3")]
    pub fn new(
        registry_path: PathBuf,
        a2a_runtime: Arc<A2ARuntime>,
        store: AgentRegistryStore,
        source: Arc<dyn RegistrySourcePort>,
    ) -> Self {
        Self {
            registry_path,
            a2a_runtime,
            store,
            source,
        }
    }

    /// expect: "The system loads and adapts agent registries for generative use" [P3]
    /// \[P3\] Motivating: Generative Space — restore previously registered agents
    /// pre:  The store schema has been initialized.
    /// post: If existing agents are found in the store, returns them
    ///       immediately (restore path). Otherwise, loads all agents from
    ///       YAML files via `load_all()`.
    #[rs::contract(id = "P3-agt-registry-loader-restore", principle = "P3")]
    #[rs::contract(id = "P3-agt-registry-loader-restore", principle = "P3")]
    pub async fn boot(&self) -> Result<Vec<RegisteredAgent>, RegistryLoaderError> {
        self.store.initialize_schema()?;

        let existing = self.store.list()?;
        if !existing.is_empty() {
            info!(
                target: "hkask.registry()",
                count = existing.len(),
                "Restored agent registry from storage"
            );
            return Ok(existing);
        }

        self.load_all().await
    }

    /// expect: "The system loads and adapts agent registries for generative use" [P3]
    /// \[P3\] Motivating: Generative Space — load agent definitions from filesystem
    /// pre:  The registry path contains valid YAML agent definitions.
    /// post: Returns `Ok(Vec<RegisteredAgent>)` with all successfully
    ///       loaded and A2A-registered agents; individual load failures
    ///       are logged and skipped.
    #[rs::contract(id = "P3-agt-registry-loader-load", principle = "P3")]
    #[rs::contract(id = "P3-agt-registry-loader-load", principle = "P3")]
    pub async fn load_all(&self) -> Result<Vec<RegisteredAgent>, RegistryLoaderError> {
        let yaml_files = self.discover_yaml_files()?;
        let mut registered = Vec::new();

        for path in &yaml_files {
            match self.load_and_register(path).await {
                Ok(agent) => {
                    info!(
                        target: "hkask.registry()",
                        name = %agent.definition.name,
                        kind = %agent.definition.agent_kind,
                        capabilities = agent.definition.capabilities.len(),
                        "Registered agent from {}",
                        path
                    );
                    registered.push(agent);
                }
                Err(e) => {
                    warn!(
                        target: "hkask.registry()",
                        path = %path,
                        error = %e,
                        "Failed to load agent YAML"
                    );
                }
            }
        }

        info!(
            target: "hkask.registry()",
            total = registered.len(),
            "Agent registry loaded"
        );

        Ok(registered)
    }

    async fn load_and_register(&self, path: &str) -> Result<RegisteredAgent, RegistryLoaderError> {
        let content = self
            .source
            .load_yaml(path)
            .map_err(|e| RegistryLoaderError::Io(Box::new(e)))?;
        let raw: RawYamlAgent =
            serde_yaml_neo::from_str(&content).map_err(|e| RegistryLoaderError::YamlParse {
                path: path.to_string(),
                source: e,
            })?;

        let definition = raw.into_agent_definition(path)?;

        let webid = WebID::from_persona(definition.name.as_bytes());

        let token = match self
            .a2a_runtime
            .register_agent(
                webid,
                definition.agent_kind,
                definition.capabilities.clone(),
            )
            .await
        {
            Ok(token) => token,
            Err(A2AError::AgentAlreadyRegistered(_)) => {
                let tokens = self.a2a_runtime.get_capabilities(&webid).await;
                tokens.into_iter().next().ok_or_else(|| {
                    RegistryLoaderError::InvalidDefinition(format!(
                        "Agent '{}' already registered but has no capability tokens in A2A runtime",
                        definition.name
                    ))
                })?
            }
            Err(e) => return Err(RegistryLoaderError::A2A(e)),
        };

        let registered = RegisteredAgent {
            definition: definition.clone(),
            token_hash: hex::encode(token.signature_bytes()),
            registered_at: now_rfc3339(),
            source_yaml: path.to_string(),
        };

        self.store.insert(&registered)?;

        Ok(registered)
    }

    fn discover_yaml_files(&self) -> Result<Vec<String>, RegistryLoaderError> {
        let registry_dir = self.registry_path.to_str().ok_or_else(|| {
            RegistryLoaderError::Io(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid registry path: {}", self.registry_path.display()),
            )))
        })?;

        // If the path doesn't exist, the source adapter will return an empty list
        // (filesystem adapter returns Io error; callers handle missing dirs gracefully)
        let files = self
            .source
            .list_yaml_files(registry_dir)
            .map_err(|e| RegistryLoaderError::Io(Box::new(e)))?;
        let mut files = files;
        files.sort();
        Ok(files)
    }

    /// expect: "The system loads and adapts agent registries for generative use" [P3]
    /// \[P8\] Motivating: Semantic Grounding — accessor for the registry store
    /// pre:  (none — accessor).
    /// post: Returns a reference to the inner `AgentRegistryStore`.
    #[rs::contract(id = "P3-agt-registry-loader-store", principle = "P3")]
    #[rs::contract(id = "P3-agt-registry-loader-store", principle = "P3")]
    pub fn store(&self) -> &AgentRegistryStore {
        &self.store
    }
}
