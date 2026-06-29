//! AgentRegistryLoader — Load agent YAML definitions, register with A2A, persist to storage

use crate::a2a::{A2AError, A2ARuntime};
use crate::adapters::registry_source::FilesystemRegistrySource;
use hkask_storage::{AgentRegistryError, AgentRegistryStore, RegisteredAgent, now_rfc3339};
use hkask_types::WebID;
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

use crate::yaml_types::RawYamlAgent;

pub struct AgentRegistryLoader {
    registry_path: PathBuf,
    a2a_runtime: Arc<A2ARuntime>,
    store: AgentRegistryStore,
    source: Arc<FilesystemRegistrySource>,
}

impl AgentRegistryLoader {
    /// expect: "The system loads and adapts agent registries for generative use"
    /// \[P3\] Motivating: Generative Space — loader reads YAML agent definitions into registry
    /// pre:  `registry_path` is a valid `PathBuf`; `a2a_runtime` is a
    ///       valid `Arc<A2ARuntime>`; `store` is a valid
    ///       `AgentRegistryStore`; `source` is a valid
    ///       `Arc<FilesystemRegistrySource>`.
    /// post: Returns an `AgentRegistryLoader` with all fields set.
    pub fn new(
        registry_path: PathBuf,
        a2a_runtime: Arc<A2ARuntime>,
        store: AgentRegistryStore,
        source: Arc<FilesystemRegistrySource>,
    ) -> Self {
        Self {
            registry_path,
            a2a_runtime,
            store,
            source,
        }
    }

    /// expect: "The system loads and adapts agent registries for generative use"
    /// \[P3\] Motivating: Generative Space — restore previously registered agents
    /// pre:  The store schema has been initialized.
    /// post: If existing agents are found in the store, returns them
    ///       immediately (restore path). Otherwise, loads all agents from
    ///       YAML files via `load_all()`.
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

    /// expect: "The system loads and adapts agent registries for generative use"
    /// \[P3\] Motivating: Generative Space — load agent definitions from filesystem
    /// pre:  The registry path contains valid YAML agent definitions.
    /// post: Returns `Ok(Vec<RegisteredAgent>)` with all successfully
    ///       loaded and A2A-registered agents; individual load failures
    ///       are logged and skipped.
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

        let definition = raw.build_definition(
            || {
                RegistryLoaderError::InvalidDefinition(format!(
                    "No 'agent:' or 'bot:' section in {}",
                    path
                ))
            },
            |agent_type| {
                RegistryLoaderError::InvalidDefinition(format!(
                    "Unknown agent type '{}' in {}",
                    agent_type, path
                ))
            },
        )?;

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
            definition,
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

    /// expect: "The system loads and adapts agent registries for generative use"
    /// \[P8\] Motivating: Semantic Grounding — accessor for the registry store
    /// pre:  (none — accessor).
    /// post: Returns a reference to the inner `AgentRegistryStore`.
    pub fn store(&self) -> &AgentRegistryStore {
        &self.store
    }
}
