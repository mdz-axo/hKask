//! ActivePods — Runtime registry for active pod deployments.
//!
//! Stores PodFactory + port references so the API matches
//! the old PodManager signature exactly. Zero consumer changes needed.

use super::AgentPodError;
use super::context::PodContext;
use super::deployment::{PodDeployment, PodFactory};
use super::types::{AgentKind, AgentPersona, PodID, PodLifecycleState};
use crate::ports::{EpisodicStoragePort, MCPRuntimePort, SemanticStoragePort};
use hkask_cns::GovernedTool;
use hkask_mcp::RawMcpToolPort;
use hkask_types::{CapabilityChecker, NuEventSink, WebID};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct ActivePods {
    deployments: RwLock<HashMap<PodID, PodDeployment>>,
    factory: Option<Arc<PodFactory>>,
    mcp_runtime: Option<Arc<dyn MCPRuntimePort>>,
    governed_tool: Option<Arc<GovernedTool<RawMcpToolPort>>>,
    capability_checker: Option<Arc<CapabilityChecker>>,
    nu_event_sink: Option<Arc<dyn NuEventSink>>,
    episodic_adapter: Option<Arc<dyn EpisodicStoragePort>>,
    semantic_adapter: Option<Arc<dyn SemanticStoragePort>>,
}

impl ActivePods {
    pub fn new() -> Self {
        Self {
            deployments: RwLock::new(HashMap::new()),
            factory: None,
            mcp_runtime: None,
            governed_tool: None,
            capability_checker: None,
            nu_event_sink: None,
            episodic_adapter: None,
            semantic_adapter: None,
        }
    }

    /// Wire the factory and port adapters so create_pod/activate_pod work
    /// with the simple old PodManager-style signatures.
    pub fn with_factory_and_ports(
        mut self,
        factory: Arc<PodFactory>,
        mcp_runtime: Arc<dyn MCPRuntimePort>,
        governed_tool: Option<Arc<GovernedTool<RawMcpToolPort>>>,
        capability_checker: Option<Arc<CapabilityChecker>>,
        nu_event_sink: Option<Arc<dyn NuEventSink>>,
        episodic_adapter: Arc<dyn EpisodicStoragePort>,
        semantic_adapter: Arc<dyn SemanticStoragePort>,
    ) -> Self {
        self.factory = Some(factory);
        self.mcp_runtime = Some(mcp_runtime);
        self.governed_tool = governed_tool;
        self.capability_checker = capability_checker;
        self.nu_event_sink = nu_event_sink;
        self.episodic_adapter = Some(episodic_adapter);
        self.semantic_adapter = Some(semantic_adapter);
        self
    }

    pub async fn insert(&self, deployment: PodDeployment) {
        self.deployments
            .write()
            .await
            .insert(deployment.pod_id, deployment);
    }

    pub async fn remove(&self, pod_id: &PodID) -> Option<PodDeployment> {
        self.deployments.write().await.remove(pod_id)
    }

    /// Get a PodContext for an active pod.
    pub async fn context(&self, pod_id: &PodID) -> Result<PodContext, AgentPodError> {
        let deployments = self.deployments.read().await;
        let deployment = deployments
            .get(pod_id)
            .ok_or(AgentPodError::PodNotFound(*pod_id))?;
        PodContext::from_deployment(deployment)
    }

    /// Find a pod by replicant name — matches old PodManager::find_pod_by_name.
    pub async fn find_by_name(&self, name: &str) -> Option<PodID> {
        let deployments = self.deployments.read().await;
        for (id, d) in deployments.iter() {
            if d.pod.persona.agent.name == name {
                return Some(*id);
            }
        }
        None
    }

    /// Alias: matches old PodManager API.
    pub async fn find_pod_by_name(&self, name: &str) -> Option<PodID> {
        self.find_by_name(name).await
    }

    /// Get a pod's WebID — matches old PodManager::get_pod_webid.
    pub async fn get_pod_webid(&self, pod_id: &PodID) -> Option<WebID> {
        self.deployments
            .read()
            .await
            .get(pod_id)
            .map(|d| d.pod.webid)
    }

    /// Alias for get_pod_webid.
    pub async fn webid(&self, pod_id: &PodID) -> Option<WebID> {
        self.get_pod_webid(pod_id).await
    }

    /// Alias for has_role — matches old PodManager::is_assigned_to_role.
    pub async fn is_assigned_to_role(&self, pod_id: &PodID, role: &str) -> bool {
        self.deployments
            .read()
            .await
            .get(pod_id)
            .map(|d| d.pod.assigned_mcp_roles.iter().any(|r| r == role))
            .unwrap_or(false)
    }

    /// Alias for is_assigned_to_role.
    pub async fn has_role(&self, pod_id: &PodID, role: &str) -> bool {
        self.is_assigned_to_role(pod_id, role).await
    }

    pub async fn has_capability(&self, pod_id: &PodID, tool: &str) -> bool {
        self.deployments
            .read()
            .await
            .get(pod_id)
            .map(|d| {
                d.pod
                    .persona
                    .capabilities
                    .iter()
                    .any(|cap| cap == tool || cap.starts_with(&format!("{}:", tool)))
            })
            .unwrap_or(false)
    }

    /// Create a pod — matches old PodManager::create_pod(template, persona, name).
    /// Uses internally stored PodFactory and port adapters.
    pub async fn create_pod(
        &self,
        template_name: &str,
        persona: &AgentPersona,
        _name: Option<String>,
    ) -> Result<PodID, AgentPodError> {
        let factory = self.factory.as_ref().ok_or_else(|| {
            AgentPodError::PersonaParseError("ActivePods not wired with PodFactory".into())
        })?;
        let mcp = Arc::clone(self.mcp_runtime.as_ref().ok_or_else(|| {
            AgentPodError::PersonaParseError("ActivePods not wired with MCP runtime".into())
        })?);
        let deployment = factory
            .deploy(
                template_name,
                persona,
                mcp,
                self.governed_tool.clone(),
                self.capability_checker.clone(),
                self.nu_event_sink.clone(),
                Arc::clone(self.episodic_adapter.as_ref().ok_or_else(|| {
                    AgentPodError::PersonaParseError(
                        "ActivePods not wired with episodic adapter".into(),
                    )
                })?),
                Arc::clone(self.semantic_adapter.as_ref().ok_or_else(|| {
                    AgentPodError::PersonaParseError(
                        "ActivePods not wired with semantic adapter".into(),
                    )
                })?),
            )
            .await
            .map_err(|e| AgentPodError::PersonaParseError(e.to_string()))?;
        let pod_id = deployment.pod_id;
        self.insert(deployment).await;
        Ok(pod_id)
    }

    /// Activate a pod — matches old PodManager::activate_pod(id).
    pub async fn activate_pod(&self, pod_id: &PodID) -> Result<(), AgentPodError> {
        let mcp = self.mcp_runtime.as_ref().ok_or_else(|| {
            AgentPodError::PersonaParseError("ActivePods not wired with MCP runtime".into())
        })?;
        let mut d = self.deployments.write().await;
        d.get_mut(pod_id)
            .ok_or(AgentPodError::PodNotFound(*pod_id))?
            .pod
            .activate(mcp.as_ref())
    }

    /// Deactivate a pod — matches old PodManager::deactivate_pod(id).
    pub async fn deactivate_pod(&self, pod_id: &PodID) -> Result<(), AgentPodError> {
        let mut d = self.deployments.write().await;
        d.get_mut(pod_id)
            .ok_or(AgentPodError::PodNotFound(*pod_id))?
            .pod
            .deactivate()
    }

    /// Get pod status — matches old PodManager::get_pod_status(id).
    pub async fn get_pod_status(&self, pod_id: &PodID) -> Result<PodStatusInfo, AgentPodError> {
        let d = self.deployments.read().await;
        let d = d.get(pod_id).ok_or(AgentPodError::PodNotFound(*pod_id))?;
        Ok(PodStatusInfo {
            pod_id: d.pod_id.to_string(),
            name: Some(d.pod.persona.agent.name.clone()),
            state: d.pod.state,
            webid: d.pod.webid.to_string(),
            agent_type: d.pod.agent_type,
            template: d.pod.template_crate.name.clone(),
            created_at: d.pod.created_at,
        })
    }

    /// List all pods — matches old PodManager::list_pods().
    pub async fn list_pods(&self) -> Result<Vec<PodStatusInfo>, AgentPodError> {
        self.deployments
            .read()
            .await
            .values()
            .map(|d| {
                Ok(PodStatusInfo {
                    pod_id: d.pod_id.to_string(),
                    name: Some(d.pod.persona.agent.name.clone()),
                    state: d.pod.state,
                    webid: d.pod.webid.to_string(),
                    agent_type: d.pod.agent_type,
                    template: d.pod.template_crate.name.clone(),
                    created_at: d.pod.created_at,
                })
            })
            .collect()
    }

    pub async fn assign_role(&self, name: &str, role: &str) -> Result<(), AgentPodError> {
        let pod_id = self.find_by_name(name).await.ok_or_else(|| {
            AgentPodError::PersonaParseError(format!("No pod found for replicant '{}'", name))
        })?;
        let mut d = self.deployments.write().await;
        let d = d
            .get_mut(&pod_id)
            .ok_or(AgentPodError::PodNotFound(pod_id))?;
        if !d.pod.assigned_mcp_roles.iter().any(|r| r == role) {
            d.pod.assigned_mcp_roles.push(role.to_string());
        }
        Ok(())
    }

    pub async fn set_mode(
        &self,
        name: &str,
        mode: &str,
        role: Option<&str>,
    ) -> Result<(), AgentPodError> {
        let pod_id = self.find_by_name(name).await.ok_or_else(|| {
            AgentPodError::PersonaParseError(format!("No pod found for replicant '{}'", name))
        })?;
        let mut d = self.deployments.write().await;
        let d = d
            .get_mut(&pod_id)
            .ok_or(AgentPodError::PodNotFound(pod_id))?;
        match mode {
            "server" => d.pod.enter_server_mode(role.ok_or_else(|| {
                AgentPodError::PersonaParseError("role required for server mode".to_string())
            })?),
            "chat" => d.pod.enter_chat_mode(),
            "exit" => d.pod.exit_mode(),
            other => Err(AgentPodError::PersonaParseError(format!(
                "Unknown mode: {}",
                other
            ))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PodStatusInfo {
    pub pod_id: String,
    pub name: Option<String>,
    pub state: PodLifecycleState,
    pub webid: String,
    pub agent_type: AgentKind,
    pub template: String,
    pub created_at: i64,
}

impl Default for ActivePods {
    fn default() -> Self {
        Self::new()
    }
}
