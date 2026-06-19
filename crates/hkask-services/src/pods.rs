//! PodService — agent pod lifecycle management for CLI and API surfaces.
//!
//! Delegates to ActivePods (runtime registry) + PodFactory.

use crate::ServiceError;
use hkask_agents::pod::{ActivePods, AgentPersona, PodID, PodStatusInfo};
use hkask_rsolidity::contract;
use hkask_services_context::AgentService;
use hkask_types::CapabilityChecker;
use std::sync::Arc;

pub struct CreatePodRequest {
    pub template: String,
    pub persona_yaml: String,
    pub name: Option<String>,
}

pub struct PodResponse {
    pub pod_id: String,
}

pub struct PodStatusResponse {
    pub pod_id: String,
    pub name: Option<String>,
    pub state: String,
    pub webid: String,
    pub agent_type: String,
    pub template: String,
    pub created_at: i64,
}

impl From<PodStatusInfo> for PodStatusResponse {
    fn from(s: PodStatusInfo) -> Self {
        Self {
            pod_id: s.pod_id,
            name: s.name,
            state: s.state.to_string(),
            webid: s.webid,
            agent_type: s.agent_type.to_string(),
            template: s.template,
            created_at: s.created_at,
        }
    }
}

/// Service for pod lifecycle management.
pub struct PodService;

impl PodService {
    #[contract(id = "P1-svc-pods-128", principle = "P1")]
    pub async fn create_pod(
        ctx: &AgentService,
        req: CreatePodRequest,
    ) -> Result<PodResponse, ServiceError> {
        let persona = AgentPersona::from_yaml(&req.persona_yaml).map_err(|e| {
            ServiceError::ValidationError {
                source: Some(Box::new(e)),
                message: format!("Invalid persona YAML: {e}"),
            }
        })?;
        let pods = ctx.active_pods();
        let factory = ctx.pod_factory();
        // Note: create_pod via ActivePods requires full port wiring.
        // For the service layer, we delegate directly.
        // Placeholder — full wiring requires MCP runtime, governed tool, etc.
        Err(ServiceError::Pod {
            message: "create_pod: full port wiring required — use PodFactory::deploy directly"
                .into(),
        })
    }

    #[contract(id = "P1-svc-pods-129", principle = "P1")]
    pub async fn list_pods(ctx: &AgentService) -> Result<Vec<PodStatusResponse>, ServiceError> {
        let pods = ctx.active_pods();
        let statuses = pods.list_statuses().await.map_err(|e| ServiceError::Pod {
            message: e.to_string(),
        })?;
        Ok(statuses.into_iter().map(PodStatusResponse::from).collect())
    }

    #[contract(id = "P1-svc-pods-130", principle = "P1")]
    pub async fn activate_pod(ctx: &AgentService, pod_id: &str) -> Result<(), ServiceError> {
        let pid = Self::parse_pod_id(pod_id)?;
        // Activation requires MCP runtime port — wired through the context's MCP runtime
        let mcp_runtime = ctx.mcp_runtime();
        let mcp_adapter = hkask_agents::adapters::mcp_runtime::FullMcpAdapter::new(
            Arc::new(hkask_types::CapabilityChecker::new(b"service-activate")),
            Arc::new((**mcp_runtime).clone()),
            tokio::runtime::Handle::current(),
        );
        ctx.active_pods()
            .activate_pod(&pid, &mcp_adapter)
            .await
            .map_err(|e| ServiceError::Pod {
                message: e.to_string(),
            })
    }

    #[contract(id = "P1-svc-pods-131", principle = "P1")]
    pub async fn deactivate_pod(ctx: &AgentService, pod_id: &str) -> Result<(), ServiceError> {
        let pid = Self::parse_pod_id(pod_id)?;
        ctx.active_pods()
            .deactivate_pod(&pid)
            .await
            .map_err(|e| ServiceError::Pod {
                message: e.to_string(),
            })
    }

    #[contract(id = "P1-svc-pods-132", principle = "P1")]
    pub async fn get_pod_status(
        ctx: &AgentService,
        pod_id: &str,
    ) -> Result<PodStatusResponse, ServiceError> {
        let pid = Self::parse_pod_id(pod_id)?;
        let status = ctx
            .active_pods()
            .get_status(&pid)
            .await
            .map_err(|e| ServiceError::Pod {
                message: e.to_string(),
            })?;
        Ok(PodStatusResponse::from(status))
    }

    fn parse_pod_id(id: &str) -> Result<PodID, ServiceError> {
        use uuid::Uuid;
        Uuid::parse_str(id)
            .map(PodID::from_uuid)
            .map_err(|_| ServiceError::PodNotFound {
                source: None,
                message: format!("Invalid pod ID '{}'", id),
            })
    }

    #[contract(id = "P1-svc-pods-133", principle = "P1")]
    pub async fn assign_role(
        ctx: &AgentService,
        name: &str,
        role: &str,
    ) -> Result<(), ServiceError> {
        ctx.active_pods()
            .assign_role(name, role)
            .await
            .map_err(|e| ServiceError::Pod {
                message: e.to_string(),
            })
    }

    #[contract(id = "P1-svc-pods-134", principle = "P1")]
    pub async fn set_mode(
        ctx: &AgentService,
        name: &str,
        mode: &str,
        role: Option<&str>,
    ) -> Result<(), ServiceError> {
        ctx.active_pods()
            .set_mode(name, mode, role)
            .await
            .map_err(|e| ServiceError::Pod {
                message: e.to_string(),
            })
    }
}
