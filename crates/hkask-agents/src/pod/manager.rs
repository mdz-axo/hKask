//! PodManager, PodStatus — Pod lifecycle management

use hkask_cns::GovernedTool;
use hkask_mcp::raw_tool_port::RawMcpToolPort;
use hkask_types::{CapabilityChecker, InferencePort, NuEventSink};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use super::types::{AgentKind, AgentPersona, PodID, PodLifecycleState};
use super::{AgentPod, AgentPodError, AgentPodResult};
use crate::adapters::mcp_runtime::CapabilityOnlyAdapter;
use crate::adapters::memory_loop_adapter::MemoryLoopAdapter;
use crate::ports::{EpisodicStoragePort, MCPRuntimePort, RecalledEpisode, SemanticStoragePort};
use hkask_mcp::GitCasAdapter;

pub struct PodManager {
    pub(crate) pods: Arc<RwLock<HashMap<PodID, AgentPod>>>,
    git_cas: Arc<GitCasAdapter>,
    acp_runtime: Arc<dyn crate::ports::AcpPort + Send + Sync>,
    pub(crate) mcp_runtime: Arc<dyn MCPRuntimePort>,
    pub(crate) episodic_storage: Arc<dyn EpisodicStoragePort>,
    pub(crate) semantic_storage: Arc<dyn SemanticStoragePort>,
    pub(crate) inference_port: Option<Arc<dyn InferencePort>>,
    pub(crate) capability_checker: Option<Arc<CapabilityChecker>>,
    pub(crate) governed_tool: Option<Arc<GovernedTool<RawMcpToolPort>>>,
    nu_event_sink: Option<Arc<dyn NuEventSink>>,
    consent: Arc<dyn crate::SovereigntyConsent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodStatus {
    pub pod_id: String,
    pub name: Option<String>,
    pub state: PodLifecycleState,
    pub webid: String,
    pub agent_type: AgentKind,
    pub template: String,
    pub created_at: i64,
}

impl PodStatus {
    fn from_pod(pod: &AgentPod) -> Self {
        Self {
            pod_id: pod.id.to_string(),
            name: Some(pod.persona.agent.name.clone()),
            state: pod.state,
            webid: pod.webid.to_string(),
            agent_type: pod.agent_type,
            template: pod.template_crate.name.clone(),
            created_at: pod.created_at,
        }
    }
}

impl PodManager {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        git_cas: Option<Arc<GitCasAdapter>>,
        acp_runtime: Option<Arc<dyn crate::ports::AcpPort + Send + Sync>>,
        mcp_runtime: Option<Arc<dyn MCPRuntimePort>>,
        episodic_storage: Option<Arc<dyn EpisodicStoragePort>>,
        semantic_storage: Option<Arc<dyn SemanticStoragePort>>,
        inference_port: Option<Arc<dyn InferencePort>>,
        capability_checker: Option<Arc<CapabilityChecker>>,
        governed_tool: Option<Arc<GovernedTool<RawMcpToolPort>>>,
        nu_event_sink: Option<Arc<dyn NuEventSink>>,
    ) -> Self {
        if episodic_storage.is_none() || semantic_storage.is_none() {
            tracing::warn!(target: "hkask.agents.pod",
                "No persistent storage configured — episodic and semantic memory are in-memory and will be lost on restart. \
                 Use PodManager::new() with explicit storage ports for sovereign persistence.");
        }
        let default_adapter = Arc::new(MemoryLoopAdapter::in_memory_unchecked());
        let capability_checker =
            capability_checker.or_else(|| resolve_acp_secret_for_checker().map(Arc::new));
        if capability_checker.is_none() {
            tracing::info!(target: "hkask.ocap",
                "No capability checker configured — PodContext::require_capability() will deny capabilities");
        }
        Self {
            pods: Arc::new(RwLock::new(HashMap::new())),
            git_cas: git_cas.unwrap_or_else(|| {
                Arc::new(GitCasAdapter::from_path(PathBuf::from(
                    "./registry/templates",
                )))
            }),
            acp_runtime: acp_runtime.unwrap_or_else(|| Arc::new(crate::acp::AcpRuntime::default())),
            mcp_runtime: mcp_runtime.unwrap_or_else(|| {
                Arc::new(CapabilityOnlyAdapter::new(Arc::new(
                    CapabilityChecker::new(&[]),
                )))
            }),
            episodic_storage: episodic_storage.unwrap_or(default_adapter.clone()),
            semantic_storage: semantic_storage.unwrap_or(default_adapter),
            inference_port,
            capability_checker,
            governed_tool,
            nu_event_sink,
            consent: Arc::new(crate::DenyAllConsent),
        }
    }

    pub fn with_consent_port(mut self, consent: Arc<dyn crate::SovereigntyConsent>) -> Self {
        self.consent = consent;
        self
    }
    pub fn with_capability_checker(mut self, checker: CapabilityChecker) -> Self {
        self.capability_checker = Some(Arc::new(checker));
        self
    }
    pub fn with_nu_event_sink(mut self, sink: Arc<dyn NuEventSink>) -> Self {
        self.nu_event_sink = Some(sink);
        self
    }
    pub fn with_governed_tool(mut self, tool: Arc<GovernedTool<RawMcpToolPort>>) -> Self {
        self.governed_tool = Some(tool);
        self
    }

    pub fn with_inference(
        git_cas: Arc<GitCasAdapter>,
        acp_runtime: Arc<dyn crate::ports::AcpPort + Send + Sync>,
        mcp_runtime: Arc<dyn MCPRuntimePort>,
        episodic_storage: Arc<dyn EpisodicStoragePort>,
        semantic_storage: Arc<dyn SemanticStoragePort>,
        inference_port: Arc<dyn InferencePort>,
    ) -> Self {
        Self::new(
            Some(git_cas),
            Some(acp_runtime),
            Some(mcp_runtime),
            Some(episodic_storage),
            Some(semantic_storage),
            Some(inference_port),
            None,
            None,
            None,
        )
    }

    pub fn inference_port(&self) -> Option<Arc<dyn InferencePort>> {
        self.inference_port.clone()
    }

    pub async fn sovereignty_checker_for(
        &self,
        pod_id: &PodID,
    ) -> Option<Arc<crate::SovereigntyChecker>> {
        self.pods
            .read()
            .await
            .get(pod_id)
            .map(|pod| Arc::new(pod.sovereignty_checker.clone()))
    }

    pub fn new_mock() -> Self {
        const MOCK_ACP_SECRET: &[u8] = b"hkask-mock-acp-secret-32-bytes!!";
        let adapter = Arc::new(MemoryLoopAdapter::in_memory_unchecked());
        let capability_checker = Arc::new(CapabilityChecker::new(MOCK_ACP_SECRET));
        Self {
            pods: Arc::new(RwLock::new(HashMap::new())),
            git_cas: Arc::new(GitCasAdapter::from_path(PathBuf::from("/tmp/hkask-mock"))),
            acp_runtime: Arc::new(crate::acp::AcpRuntime::new(MOCK_ACP_SECRET)),
            mcp_runtime: Arc::new(CapabilityOnlyAdapter::new(Arc::clone(&capability_checker))),
            episodic_storage: adapter.clone(),
            semantic_storage: adapter,
            inference_port: None,
            capability_checker: Some(capability_checker),
            governed_tool: None,
            nu_event_sink: None,
            consent: Arc::new(crate::DenyAllConsent),
        }
    }
}

impl PodManager {
    pub async fn create_pod(
        &self,
        template_name: &str,
        persona: &AgentPersona,
        name: Option<String>,
    ) -> AgentPodResult<PodID> {
        AgentPersona::validate_fields(
            &persona.agent.name,
            &persona.agent.agent_type.to_string().to_lowercase(),
            &persona.agent.version,
            &persona.charter.description,
            &persona.charter.editor,
            &persona.capabilities,
        )?;
        let pod = AgentPod::new(
            template_name,
            persona,
            self.git_cas.as_ref(),
            Arc::clone(&self.consent),
        )?;
        let pod_id = pod.id;
        self.pods.write().await.insert(pod_id, pod);
        info!(target: "hkask.pod", pod_id = %pod_id, template = %template_name, name = ?name, "Pod created");
        Ok(pod_id)
    }

    pub async fn activate_pod(&self, pod_id: &PodID) -> AgentPodResult<()> {
        let registration_data = {
            let pods = self.pods.read().await;
            let pod = pods
                .get(pod_id)
                .ok_or_else(|| AgentPodError::PodNotFound(*pod_id))?;
            if pod.state() == PodLifecycleState::Populated {
                Some((pod.webid, pod.agent_type, pod.persona.capabilities.clone()))
            } else {
                None
            }
        };
        let token = if let Some((webid, agent_type, capabilities)) = registration_data {
            Some(
                self.acp_runtime
                    .register_agent(webid, agent_type, capabilities)
                    .await?,
            )
        } else {
            None
        };

        let mut pods = self.pods.write().await;
        let pod = pods
            .get_mut(pod_id)
            .ok_or_else(|| AgentPodError::PodNotFound(*pod_id))?;
        if let Some(token) = token {
            pod.capability_token = token;
            pod.state = PodLifecycleState::Registered;
            tracing::debug!(target: "cns.pod", span = "cns.agent_pod.registered", verb = "registered",
                pod_id = %pod.id, webid = %pod.webid, agent_type = %pod.agent_type, confidence = 1.0, "CNS event");
            info!("Agent pod {} registered with ACP", pod.id);
            if let Some(ref sink) = self.nu_event_sink {
                crate::pod::nu_event::emit_pod_registered(
                    sink.as_ref(),
                    pod.webid,
                    &pod.id.to_string(),
                    &pod.agent_type.to_string(),
                );
            }
        }
        pod.activate(self.mcp_runtime.as_ref())?;
        if let Some(ref sink) = self.nu_event_sink {
            crate::pod::nu_event::emit_pod_activated(sink.as_ref(), pod.webid, &pod.id.to_string());
        }
        info!(target: "hkask.pod", pod_id = %pod_id, "Pod activated");
        Ok(())
    }

    pub async fn deactivate_pod(&self, pod_id: &PodID) -> AgentPodResult<()> {
        let mut pods = self.pods.write().await;
        let pod = pods
            .get_mut(pod_id)
            .ok_or_else(|| AgentPodError::PodNotFound(*pod_id))?;
        let token_id = pod.capability_token.id.clone();
        let webid = pod.webid;
        pod.deactivate()?;
        if let Some(ref sink) = self.nu_event_sink {
            crate::pod::nu_event::emit_pod_deactivated(
                sink.as_ref(),
                pod.webid,
                &pod.id.to_string(),
            );
        }
        if let Err(e) = self.acp_runtime.revoke_capability(&token_id, &webid).await {
            tracing::warn!(target: "hkask.pod", pod_id = %pod_id, token_id = %token_id, error = %e,
                "Failed to revoke capability token on deactivation (pod is still deactivated)");
            tracing::debug!(target: "cns.pod", span = "cns.agent_pod.revocation_warning",
                verb = "revocation_warning", pod_id = %pod_id, token_id = %token_id, error = %e,
                confidence = 0.8, "CNS event");
        }
        info!(target: "hkask.pod", pod_id = %pod_id, "Pod deactivated");
        Ok(())
    }

    pub async fn recall_pod_events(&self, pod_id: &PodID) -> AgentPodResult<Vec<RecalledEpisode>> {
        let pods = self.pods.read().await;
        let pod = pods
            .get(pod_id)
            .ok_or_else(|| AgentPodError::PodNotFound(*pod_id))?;
        self.episodic_storage
            .recall_episodic(&crate::ports::RecallRequest::episodic(
                "lifecycle",
                pod.webid,
                pod.capability_token.clone(),
            ))
            .map_err(AgentPodError::from)
    }

    pub async fn get_pod_status(&self, pod_id: &PodID) -> AgentPodResult<PodStatus> {
        self.pods
            .read()
            .await
            .get(pod_id)
            .map(|p| PodStatus::from_pod(p))
            .ok_or_else(|| AgentPodError::PodNotFound(*pod_id))
    }

    pub async fn list_pods(&self) -> AgentPodResult<Vec<PodStatus>> {
        Ok(self
            .pods
            .read()
            .await
            .values()
            .map(PodStatus::from_pod)
            .collect())
    }

    pub fn acp_runtime(&self) -> Arc<dyn crate::ports::AcpPort + Send + Sync> {
        Arc::clone(&self.acp_runtime)
    }
}

impl Default for PodManager {
    fn default() -> Self {
        Self::new_mock()
    }
}

fn resolve_acp_secret_for_checker() -> Option<CapabilityChecker> {
    hkask_keystore::resolve_acp_secret()
        .ok()
        .map(|secret| CapabilityChecker::new(&secret))
}
