//! ActivePods — Runtime registry for active pod deployments.

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
}

impl ActivePods {
    pub fn new() -> Self {
        Self {
            deployments: RwLock::new(HashMap::new()),
        }
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

    pub async fn context(&self, pod_id: &PodID) -> Result<PodContext, AgentPodError> {
        let deployments = self.deployments.read().await;
        let deployment = deployments
            .get(pod_id)
            .ok_or(AgentPodError::PodNotFound(*pod_id))?;
        PodContext::from_deployment(deployment)
    }

    pub async fn find_by_name(&self, name: &str) -> Option<PodID> {
        let deployments = self.deployments.read().await;
        for (id, d) in deployments.iter() {
            if d.pod.persona.agent.name == name {
                return Some(*id);
            }
        }
        None
    }

    pub async fn list_ids(&self) -> Vec<PodID> {
        self.deployments.read().await.keys().copied().collect()
    }

    pub async fn has_role(&self, pod_id: &PodID, role: &str) -> bool {
        self.deployments
            .read()
            .await
            .get(pod_id)
            .map(|d| d.pod.assigned_mcp_roles.iter().any(|r| r == role))
            .unwrap_or(false)
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

    pub async fn webid(&self, pod_id: &PodID) -> Option<WebID> {
        self.deployments
            .read()
            .await
            .get(pod_id)
            .map(|d| d.pod.webid)
    }

    pub async fn create_pod(
        &self,
        factory: &PodFactory,
        template_name: &str,
        persona: &AgentPersona,
        mcp_runtime: Arc<dyn MCPRuntimePort>,
        governed_tool: Option<Arc<GovernedTool<RawMcpToolPort>>>,
        capability_checker: Option<Arc<CapabilityChecker>>,
        nu_event_sink: Option<Arc<dyn NuEventSink>>,
        episodic_adapter: Arc<dyn EpisodicStoragePort>,
        semantic_adapter: Arc<dyn SemanticStoragePort>,
    ) -> Result<PodID, AgentPodError> {
        let deployment = factory
            .deploy(
                template_name,
                persona,
                mcp_runtime,
                governed_tool,
                capability_checker,
                nu_event_sink,
                episodic_adapter,
                semantic_adapter,
            )
            .await
            .map_err(|e| AgentPodError::PersonaParseError(e.to_string()))?;
        let pod_id = deployment.pod_id;
        self.insert(deployment).await;
        Ok(pod_id)
    }

    pub async fn activate_pod(
        &self,
        pod_id: &PodID,
        mcp: &dyn MCPRuntimePort,
    ) -> Result<(), AgentPodError> {
        let mut d = self.deployments.write().await;
        d.get_mut(pod_id)
            .ok_or(AgentPodError::PodNotFound(*pod_id))?
            .pod
            .activate(mcp)
    }

    pub async fn deactivate_pod(&self, pod_id: &PodID) -> Result<(), AgentPodError> {
        let mut d = self.deployments.write().await;
        d.get_mut(pod_id)
            .ok_or(AgentPodError::PodNotFound(*pod_id))?
            .pod
            .deactivate()
    }

    pub async fn get_status(&self, pod_id: &PodID) -> Result<PodStatusInfo, AgentPodError> {
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

    pub async fn list_statuses(&self) -> Result<Vec<PodStatusInfo>, AgentPodError> {
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
