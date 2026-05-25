//! BotRegistryLoader — Load agent YAML definitions, register with ACP, persist to storage

use crate::acp::{AcpError, AcpRuntime};
use hkask_storage::{AgentRegistryError, AgentRegistryStore};
use hkask_types::{AgentDefinition, AgentKind, RegisteredAgent, WebID};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tracing::{info, warn};

#[derive(Error, Debug)]
pub enum RegistryLoaderError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML parse error in {path}: {source}")]
    YamlParse {
        path: String,
        source: serde_yaml::Error,
    },
    #[error("ACP error: {0}")]
    Acp(#[from] AcpError),
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
    binding_contract: bool,
    #[serde(default)]
    editor: String,
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
struct YamlReadinessProbe {
    #[serde(rename = "type")]
    probe_type: String,
    endpoint: String,
    #[serde(default)]
    expected: HashMap<String, serde_json::Value>,
    #[serde(default = "default_timeout")]
    timeout_seconds: u64,
    #[serde(default = "default_retry")]
    retry_count: u32,
}

fn default_timeout() -> u64 {
    15
}
fn default_retry() -> u32 {
    3
}

#[derive(Debug, Deserialize)]
struct YamlStandingSession {
    session_id: String,
    role: String,
    #[serde(default)]
    report_interval: String,
    #[serde(default)]
    administrator_visible: bool,
}

#[derive(Debug, Deserialize)]
struct YamlReporting {
    #[serde(default)]
    escalate_to: Option<String>,
    #[serde(default)]
    report_to: Option<String>,
    #[serde(default)]
    report_format: Option<String>,
    #[serde(default)]
    alert_threshold: Option<String>,
    #[serde(default)]
    report_interval: Option<String>,
    #[serde(default)]
    report_on: Vec<String>,
    #[serde(default)]
    receives_from: Vec<String>,
    #[serde(default)]
    escalation_triggers: Vec<String>,
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
    reporting: Option<YamlReporting>,
    #[serde(default)]
    standing_session: Option<YamlStandingSession>,
    #[serde(default)]
    persona: Option<YamlPersona>,
    #[serde(default)]
    depends_on: Vec<String>,
    #[serde(default)]
    readiness_probe: Option<YamlReadinessProbe>,
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

        Ok(AgentDefinition {
            name: header.name.clone(),
            agent_kind,
            binding_contract: header.binding_contract,
            editor: header.editor.clone(),
            charter: self.charter.map(|c| hkask_types::Charter {
                description: c.description,
                archetype: c.archetype,
                visibility: c.visibility,
            }),
            capabilities: self.capabilities,
            rights: Self::convert_rights(self.rights),
            responsibilities: Self::convert_responsibilities(self.responsibilities),
            reporting: self.reporting.map(|r| hkask_types::ReportingConfig {
                escalate_to: r.escalate_to,
                report_to: r.report_to,
                report_format: r.report_format,
                alert_threshold: r.alert_threshold,
                report_interval: r.report_interval,
                report_on: r.report_on,
                receives_from: r.receives_from,
                escalation_triggers: r.escalation_triggers,
            }),
            standing_session: self.standing_session.map(|s| {
                hkask_types::AgentStandingSessionConfig {
                    session_id: s.session_id,
                    role: s.role,
                    report_interval: s.report_interval,
                    administrator_visible: s.administrator_visible,
                }
            }),
            persona: self.persona.map(|p| hkask_types::PersonaConstraints {
                tone: p.tone,
                verbosity: p.verbosity,
                formatting: p.formatting,
                forbidden: p.forbidden,
                required: p.required,
            }),
            depends_on: self.depends_on,
            readiness_probe: self.readiness_probe.map(|rp| hkask_types::ReadinessProbe {
                probe_type: rp.probe_type,
                endpoint: rp.endpoint,
                expected: rp.expected,
                timeout_seconds: rp.timeout_seconds,
                retry_count: rp.retry_count,
            }),
            process_manifest: self.process_manifest,
        })
    }
}

pub struct BotRegistryLoader {
    registry_path: PathBuf,
    acp_runtime: Arc<AcpRuntime>,
    store: AgentRegistryStore,
}

impl BotRegistryLoader {
    pub fn new(
        registry_path: PathBuf,
        acp_runtime: Arc<AcpRuntime>,
        store: AgentRegistryStore,
    ) -> Self {
        Self {
            registry_path,
            acp_runtime,
            store,
        }
    }

    pub async fn boot(&self) -> Result<Vec<RegisteredAgent>, RegistryLoaderError> {
        self.store.initialize_schema()?;

        let existing = self.store.list()?;
        if !existing.is_empty() {
            info!(
                target: "hkask.registry",
                count = existing.len(),
                "Restored agent registry from storage"
            );
            return Ok(existing);
        }

        self.load_all().await
    }

    pub async fn load_all(&self) -> Result<Vec<RegisteredAgent>, RegistryLoaderError> {
        let yaml_files = self.discover_yaml_files()?;
        let mut registered = Vec::new();

        for path in &yaml_files {
            match self.load_and_register(path).await {
                Ok(agent) => {
                    info!(
                        target: "hkask.registry",
                        name = %agent.definition.name,
                        kind = %agent.definition.agent_kind,
                        capabilities = agent.definition.capabilities.len(),
                        "Registered agent from {}",
                        path.display()
                    );
                    registered.push(agent);
                }
                Err(e) => {
                    warn!(
                        target: "hkask.registry",
                        path = %path.display(),
                        error = %e,
                        "Failed to load agent YAML"
                    );
                }
            }
        }

        info!(
            target: "hkask.registry",
            total = registered.len(),
            "Agent registry loaded"
        );

        Ok(registered)
    }

    async fn load_and_register(&self, path: &Path) -> Result<RegisteredAgent, RegistryLoaderError> {
        let content = std::fs::read_to_string(path)?;
        let raw: RawYamlAgent =
            serde_yaml::from_str(&content).map_err(|e| RegistryLoaderError::YamlParse {
                path: path.display().to_string(),
                source: e,
            })?;

        let definition = raw.into_agent_definition(&path.display().to_string())?;

        let webid = WebID::from_persona(definition.name.as_bytes());

        let token = match self
            .acp_runtime
            .register_agent(
                webid,
                definition.agent_kind.as_str().to_string(),
                definition.capabilities.clone(),
            )
            .await
        {
            Ok(token) => token,
            Err(AcpError::AgentAlreadyRegistered(_)) => {
                let tokens = self.acp_runtime.get_capabilities(&webid).await;
                tokens.into_iter().next().ok_or_else(|| {
                    RegistryLoaderError::InvalidDefinition(format!(
                        "Agent '{}' already registered but has no capability tokens in ACP runtime",
                        definition.name
                    ))
                })?
            }
            Err(e) => return Err(RegistryLoaderError::Acp(e)),
        };

        let registered = RegisteredAgent {
            definition: definition.clone(),
            token_hash: token.signature.clone(),
            registered_at: chrono::Utc::now().to_rfc3339(),
            source_yaml: path.display().to_string(),
        };

        self.store.insert(&registered)?;

        Ok(registered)
    }

    fn discover_yaml_files(&self) -> Result<Vec<PathBuf>, RegistryLoaderError> {
        if !self.registry_path.exists() {
            return Ok(Vec::new());
        }

        let mut files = Vec::new();
        for entry in std::fs::read_dir(&self.registry_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("yaml")
                || path.extension().and_then(|e| e.to_str()) == Some("yml")
            {
                files.push(path);
            }
        }
        files.sort();
        Ok(files)
    }

    pub fn store(&self) -> &AgentRegistryStore {
        &self.store
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_storage::AgentRegistryStore;
    use rusqlite::Connection;
    use std::sync::{Arc, Mutex};

    fn test_loader(yaml_dir: &Path) -> BotRegistryLoader {
        let acp = Arc::new(AcpRuntime::new(b"test-secret-for-loader", None));
        let conn = Connection::open_in_memory().unwrap();
        let store = AgentRegistryStore::new(Arc::new(Mutex::new(conn)));
        BotRegistryLoader::new(yaml_dir.to_path_buf(), acp, store)
    }

    #[tokio::test]
    async fn test_load_curator_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let yaml = r#"
agent:
  name: Curator
  type: Replicant
  binding_contract: true
  editor: admin

charter:
  description: System metacognition
  archetype: MaintenanceAdvisory
  visibility: Primary

capabilities:
  - tool:cns:emit
  - tool:memory:recall

rights:
  - read: all_public_semantic_memory

responsibilities:
  - monitor: system_health

persona:
  tone: Direct
  verbosity: Minimal
  forbidden:
    - preamble
    - emojis
"#;
        std::fs::write(dir.path().join("Curator.yaml"), yaml).unwrap();

        let loader = test_loader(dir.path());
        loader.store.initialize_schema().unwrap();
        let agents = loader.load_all().await.unwrap();

        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].definition.name, "Curator");
        assert_eq!(agents[0].definition.agent_kind, AgentKind::Replicant);
        assert_eq!(agents[0].definition.capabilities.len(), 2);
        assert!(agents[0].definition.persona.is_some());
    }

    #[tokio::test]
    async fn test_load_bot_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let yaml = r#"
bot:
  name: cns-curator-bot
  type: Bot
  binding_contract: true
  editor: admin

capabilities:
  - tool:cns:emit
  - tool:cns:variety

rights:
  - read: cns_spans

responsibilities:
  - emit: cns.agent_pod.activated

readiness_probe:
  type: health_check
  endpoint: cns::variety_counter_status
  expected:
    variety_counter_available: true
  timeout_seconds: 10
  retry_count: 3
"#;
        std::fs::write(dir.path().join("cns-curator-bot.yaml"), yaml).unwrap();

        let loader = test_loader(dir.path());
        loader.store.initialize_schema().unwrap();
        let agents = loader.load_all().await.unwrap();

        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].definition.name, "cns-curator-bot");
        assert_eq!(agents[0].definition.agent_kind, AgentKind::Bot);
        assert!(agents[0].definition.readiness_probe.is_some());
    }

    #[tokio::test]
    async fn test_load_multiple_agents() {
        let dir = tempfile::tempdir().unwrap();

        let curator = r#"
agent:
  name: Curator
  type: Replicant
capabilities:
  - tool:cns:emit
"#;
        let bot = r#"
bot:
  name: test-bot
  type: Bot
capabilities:
  - tool:memory:recall
"#;
        std::fs::write(dir.path().join("Curator.yaml"), curator).unwrap();
        std::fs::write(dir.path().join("test-bot.yaml"), bot).unwrap();

        let loader = test_loader(dir.path());
        loader.store.initialize_schema().unwrap();
        let agents = loader.load_all().await.unwrap();

        assert_eq!(agents.len(), 2);
    }

    #[tokio::test]
    async fn test_boot_restores_from_storage() {
        let dir = tempfile::tempdir().unwrap();
        let yaml = r#"
bot:
  name: persistent-bot
  type: Bot
capabilities:
  - tool:cns:emit
"#;
        std::fs::write(dir.path().join("bot.yaml"), yaml).unwrap();

        let acp = Arc::new(AcpRuntime::new(b"test-secret-boot", None));
        let conn = Connection::open_in_memory().unwrap();
        let store = AgentRegistryStore::new(Arc::new(Mutex::new(conn)));

        let loader = BotRegistryLoader::new(dir.path().to_path_buf(), acp.clone(), store);
        let agents = loader.boot().await.unwrap();
        assert_eq!(agents.len(), 1);

        let loader2 = BotRegistryLoader::new(dir.path().to_path_buf(), acp, loader.store().clone());
        let agents2 = loader2.boot().await.unwrap();
        assert_eq!(agents2.len(), 1);
        assert_eq!(agents2[0].definition.name, "persistent-bot");
    }

    #[tokio::test]
    async fn test_empty_registry_path() {
        let dir = tempfile::tempdir().unwrap();
        let empty = dir.path().join("nonexistent");
        let loader = test_loader(&empty);
        loader.store.initialize_schema().unwrap();
        let agents = loader.load_all().await.unwrap();
        assert!(agents.is_empty());
    }
}
